use super::*;
use crate::{LookupError, Provider, RegistryFailure, RetryAfter};
use jiff::civil::Date;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

fn date() -> Date {
    Date::new(2025, 2, 14).unwrap()
}
fn fixture() -> Value {
    json!({"cod":200,"message":"SUCCESS","found":[{
        "date_generale":{"cui":1234,"data":"2025-02-14","denumire":"EXEMPLU SRL","adresa":"  Strada Exemplu 1\nBucurești  ","nrRegCom":"J00/1/2000","statusRO_e_Factura":false},
        "inregistrare_scop_Tva":{"scpTVA":false},
        "stare_inactiv":{"statusInactivi":true},
        "adresa_domiciliu_fiscal":{"dcod_Postal":"012345","ddenumire_Localitate":"București"}
    }],"notFound":[]})
}
fn decode(value: Value) -> Result<Option<RoCompanyLookupResult>, RegistryFailure> {
    super::decode::decode(
        &serde_json::to_vec(&value).unwrap(),
        &RoBusinessId::parse("001234").unwrap(),
        date(),
    )
}
#[test]
fn retains_identity_source_text_and_unknown_statuses() {
    let record = decode(fixture()).unwrap().unwrap();
    assert_eq!(record.tax_id.as_str(), "001234");
    assert_eq!(
        record.full_fiscal_address.as_deref(),
        Some("  Strada Exemplu 1\nBucurești  ")
    );
    assert_eq!(record.tax_status.vat_registered, Some(false));
    assert_eq!(record.tax_status.inactive, Some(true));
    assert_eq!(record.tax_status.vat_on_collection, None);
    assert_eq!(record.tax_status.e_invoice_registered, Some(false));
    assert!(record.registered_office.is_none());
    assert_eq!(
        record.fiscal_address.as_ref().unwrap().postcode.as_deref(),
        Some("012345")
    );
    let wire = serde_json::to_value(record).unwrap();
    assert_eq!(wire["source"], "RomanianAnaf");
    assert_eq!(wire["taxStatus"]["vatOnCollection"], Value::Null);
    assert!(wire.get("valid").is_none());
    assert_eq!(wire["registryUrl"], serde_json::Value::Null);
    assert_eq!(wire["foreignName"], serde_json::Value::Null);
    assert!(wire.get("details").is_none());
}
#[test]
fn only_exact_negative_envelopes_are_misses() {
    assert!(
        decode(json!({"cod":200,"message":"SUCCESS","found":[],"notFound":[1234]}))
            .unwrap()
            .is_none()
    );
    for bad in [
        json!({"cod":200,"message":"SUCCESS","found":[],"notFound":[]}),
        json!({"cod":500,"message":"ERROR","found":[],"notFound":[1234]}),
        json!({"cod":200,"message":"SUCCESS","found":[],"notFound":[9999]}),
        json!({"cod":200,"message":"SUCCESS","found":[],"notFound":[1234,1234]}),
        json!({"cod":200,"message":"SUCCESS","notFound":[1234]}),
    ] {
        assert!(decode(bad).is_err());
    }
    let mut value = fixture();
    value["notFound"] = json!([1234]);
    assert!(decode(value).is_err());
}
#[test]
fn wrong_identity_date_types_and_missing_name_fail_closed() {
    for (path, replacement) in [
        ("/found/0/date_generale/cui", json!(1235)),
        ("/found/0/date_generale/cui", json!(1234.0)),
        ("/found/0/date_generale/data", json!("2025-02-15")),
        ("/found/0/date_generale/denumire", json!(" ")),
        ("/found/0/inregistrare_scop_Tva/scpTVA", json!("false")),
        ("/found/0/adresa_domiciliu_fiscal/dcod_Postal", json!(12345)),
    ] {
        let mut value = fixture();
        *value.pointer_mut(path).unwrap() = replacement;
        assert!(decode(value).is_err(), "{path}");
    }
    let mut value = fixture();
    value["found"][0]["date_generale"]["cui"] = json!("001234");
    assert!(decode(value).is_ok());
}

async fn server(
    status: &str,
    headers: &str,
    body: String,
) -> (String, crate::test_support::http::ServerTask<String>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/tva", listener.local_addr().unwrap());
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}",
        body.len()
    );
    let task = crate::test_support::http::spawn_server(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let request = crate::test_support::http::read_request(&mut socket)
            .await
            .unwrap();
        socket.write_all(response.as_bytes()).await.unwrap();
        String::from_utf8(request).unwrap()
    });
    (endpoint, task)
}
#[tokio::test]
async fn sends_one_numeric_cui_and_requested_date() {
    let (url, task) = server(
        "200 OK",
        "Content-Type: application/json; charset=utf-8\r\n",
        fixture().to_string(),
    )
    .await;
    let client = RoRegistryClient::for_test(url, RoRegistryConfig::default());
    assert_eq!(
        client.lookup_at("001234", date()).await.unwrap_err().id,
        crate::ErrorId::UnresolvedData
    );
    let request = task.await.unwrap();
    assert!(request.starts_with("POST /tva HTTP/1.1"));
    let body: Value = serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(body, json!([{"cui":1234,"data":"2025-02-14"}]));
}
#[tokio::test]
async fn typed_negative_and_http_failures_never_become_false() {
    for (status, body, expected) in [
        (
            "200 OK",
            json!({"cod":200,"message":"SUCCESS","found":[],"notFound":[1234]}).to_string(),
            "miss",
        ),
        ("404 Not Found", String::new(), "http"),
        ("403 Forbidden", String::new(), "access"),
        ("503 Unavailable", String::new(), "unavailable"),
        ("302 Found", String::new(), "http"),
        ("200 OK", "not JSON".into(), "schema"),
    ] {
        let (url, task) = server(status, "Content-Type: application/json\r\n", body).await;
        let err = RoRegistryClient::for_test(url, RoRegistryConfig::default())
            .lookup_at("1234", date())
            .await
            .unwrap_err();
        task.await.unwrap();
        match (expected, err) {
            (
                "miss",
                LookupError {
                    kind:
                        crate::model::LookupFailure::RegistryNotFound {
                            provider: Provider::RomanianAnaf,
                        },
                    ..
                },
            ) => {}
            (
                "http",
                LookupError {
                    kind:
                        crate::model::LookupFailure::Registry {
                            error: RegistryFailure::Http { .. },
                            ..
                        },
                    ..
                },
            ) => {}
            (
                "access",
                LookupError {
                    kind:
                        crate::model::LookupFailure::Registry {
                            error: RegistryFailure::AccessDenied,
                            ..
                        },
                    ..
                },
            ) => {}
            (
                "schema",
                LookupError {
                    kind:
                        crate::model::LookupFailure::Registry {
                            error: RegistryFailure::UnexpectedResponse,
                            ..
                        },
                    ..
                },
            ) => {}
            ("unavailable", e) if e.is_service_unavailable() => {}
            (_, e) => panic!("unexpected {expected}: {e}"),
        }
    }
}
#[tokio::test]
async fn throttle_cools_down_clones_without_another_request() {
    let (url, task) = server(
        "429 Too Many Requests",
        "Retry-After: 120\r\n",
        String::new(),
    )
    .await;
    let client = RoRegistryClient::for_test(url, RoRegistryConfig::default());
    assert!(matches!(
        client.lookup_at("1234", date()).await,
        Err(LookupError {
            kind: crate::model::LookupFailure::Registry {
                error: RegistryFailure::RateLimited {
                    retry_after: Some(RetryAfter::Seconds(120))
                },
                ..
            },
            ..
        })
    ));
    task.await.unwrap();
    assert!(matches!(
        client.clone().lookup_at("1234", date()).await,
        Err(LookupError {
            kind: crate::model::LookupFailure::Registry {
                error: RegistryFailure::CoolingDown { .. },
                ..
            },
            ..
        })
    ));
}
#[tokio::test]
async fn content_type_size_and_admission_are_bounded() {
    let (url, task) = server(
        "200 OK",
        "Content-Type: text/html\r\n",
        fixture().to_string(),
    )
    .await;
    assert!(matches!(
        RoRegistryClient::for_test(url, RoRegistryConfig::default())
            .lookup_at("1234", date())
            .await,
        Err(LookupError {
            kind: crate::model::LookupFailure::Registry {
                error: RegistryFailure::UnexpectedResponse,
                ..
            },
            ..
        })
    ));
    task.await.unwrap();
    let (url, task) = server(
        "200 OK",
        "Content-Type: application/json\r\n",
        fixture().to_string(),
    )
    .await;
    let config = RoRegistryConfig {
        max_response_bytes: 10,
        raw_debug: false,
        ..Default::default()
    };
    assert!(matches!(
        RoRegistryClient::for_test(url, config)
            .lookup_at("1234", date())
            .await,
        Err(LookupError {
            kind: crate::model::LookupFailure::Registry {
                error: RegistryFailure::ResponseTooLarge,
                ..
            },
            ..
        })
    ));
    task.await.unwrap();
    let (url, task) = server(
        "200 OK",
        "Content-Type: application/json\r\n",
        fixture().to_string(),
    )
    .await;
    let mut config = RoRegistryConfig::default();
    config.timeout = Duration::from_millis(500);
    config.connect_timeout = config.timeout;
    let client = RoRegistryClient::for_test(url, config);
    assert_eq!(
        client.lookup_at("1234", date()).await.unwrap_err().id,
        crate::ErrorId::UnresolvedData
    );
    task.await.unwrap();
    assert!(matches!(
        client.lookup_at("1234", date()).await,
        Err(LookupError {
            kind: crate::model::LookupFailure::Busy,
            ..
        })
    ));
}
#[tokio::test]
async fn deadline_releases_capacity_and_establishes_cooldown() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut config = RoRegistryConfig::default();
    config.timeout = Duration::from_millis(100);
    config.connect_timeout = config.timeout;
    let client =
        RoRegistryClient::for_test(format!("http://{}", listener.local_addr().unwrap()), config);
    let clone = client.clone();
    let task = tokio::spawn(async move { clone.lookup_at("1234", date()).await });
    let (_socket, _) = listener.accept().await.unwrap();
    assert!(matches!(
        client.lookup_at("1234", date()).await,
        Err(LookupError {
            kind: crate::model::LookupFailure::Busy,
            ..
        })
    ));
    assert!(task.await.unwrap().unwrap_err().is_service_unavailable());
    assert!(matches!(
        client.lookup_at("1234", date()).await,
        Err(LookupError {
            kind: crate::model::LookupFailure::Registry {
                error: RegistryFailure::CoolingDown { .. },
                ..
            },
            ..
        })
    ));
}

#[test]
fn http_success_envelope_may_omit_documented_status_fields() {
    let mut value = fixture();
    value.as_object_mut().unwrap().remove("cod");
    value.as_object_mut().unwrap().remove("message");
    assert!(decode(value.clone()).unwrap().is_some());
    value["cod"] = json!(500);
    assert!(decode(value).is_err());
    assert!(
        decode(json!({"found":[],"notFound":[1234]}))
            .unwrap()
            .is_none()
    );
}

#[test]
fn company_status_is_shared_and_never_inferred_from_fiscal_inactivity() {
    assert_eq!(decode(fixture()).unwrap().unwrap().status, None);
    for (raw, expected) in [
        ("NECUNOSCUT", crate::CompanyStatus::Unknown),
        ("INREGISTRAT", crate::CompanyStatus::Active),
        ("RADIAT", crate::CompanyStatus::Closed),
    ] {
        let mut value = fixture();
        value["found"][0]["date_generale"]["stare_inregistrare"] = json!(raw);
        let record = decode(value).unwrap().unwrap();
        assert_eq!(record.status, Some(expected));
        assert_eq!(record.registration_status.as_deref(), Some(raw));
        assert_eq!(record.tax_status.inactive, Some(true));
        assert_eq!(
            serde_json::to_value(record).unwrap()["status"],
            serde_json::to_value(expected).unwrap()
        );
    }
}

#[test]
fn native_registry_status_keeps_exact_anaf_text_and_missing_value() {
    let missing = decode(fixture()).unwrap().unwrap();
    let shared: crate::CompanyLookupResult = missing.clone().into();
    crate::test_support::assert_company_contract(&shared);
    let wire = serde_json::to_value(shared).unwrap();
    assert!(wire.get("nativeRegistryStatus").is_none());
    assert_eq!(
        wire["registryDetails"]["data"].get("nativeRegistryStatus"),
        Some(&Value::Null)
    );
    assert!(missing.native_registry_status.is_none());
    assert!(serde_json::to_value(missing).unwrap()["nativeRegistryStatus"].is_null());
    let mut source = fixture();
    source["found"][0]["date_generale"]["stare_inregistrare"] = json!("  STARE NECUNOSCUTĂ  ");
    let record = decode(source).unwrap().unwrap();
    assert_eq!(
        record.native_registry_status.as_deref(),
        Some("  STARE NECUNOSCUTĂ  ")
    );
    assert_eq!(record.native_registry_status, record.registration_status);
    let shared: crate::CompanyLookupResult = record.clone().into();
    crate::test_support::assert_company_contract(&shared);
    let wire = serde_json::to_value(shared).unwrap();
    assert!(wire.get("nativeRegistryStatus").is_none());
    assert_eq!(
        wire["registryDetails"]["data"]["nativeRegistryStatus"],
        "  STARE NECUNOSCUTĂ  "
    );
    assert_eq!(wire["status"], "unknown");
    assert_eq!(
        serde_json::to_value(record).unwrap()["nativeRegistryStatus"],
        "  STARE NECUNOSCUTĂ  "
    );
}

#[test]
fn explicit_null_status_metadata_is_not_treated_as_omission() {
    for key in ["cod", "message"] {
        let mut value = fixture();
        value[key] = serde_json::Value::Null;
        assert!(decode(value).is_err());
    }
}

#[test]
fn shared_company_preserves_fiscal_semantics_and_source_components() {
    let native = decode(fixture()).unwrap().unwrap();
    let shared: crate::CompanyLookupResult = native.clone().into();
    crate::test_support::assert_company_contract(&shared);
    assert_eq!(shared.tax_id, "001234");
    assert_eq!(shared.country, "RO");
    assert!(shared.registered_office.is_none());
    assert!(shared.status.is_none()); // Fiscal inactivity is not legal closure.
    let fiscal = shared.fiscal_address.as_ref().unwrap();
    assert_eq!(fiscal.full_address_by_registry, native.full_fiscal_address);
    assert_eq!(fiscal.postcode.as_deref(), Some("012345"));
    assert_eq!(fiscal.country_iso2.as_deref(), Some("RO"));
    let crate::CompanyDetails::RomanianAnaf { details, .. } = &shared.registry_details else {
        panic!("wrong details")
    };
    assert_eq!(details.fiscal_address, native.fiscal_address);
    assert_eq!(details.tax_status, native.tax_status);
    let wire = serde_json::to_value(shared).unwrap();
    assert!(wire.get("valid").is_none());
    assert_eq!(wire["registryUrl"], serde_json::Value::Null);
    assert_eq!(wire["foreignName"], serde_json::Value::Null);
    let expected_keys = [
        "companyName",
        "country",
        "fiscalAddress",
        "foreignName",
        "legalForm",
        "registeredOffice",
        "registryDetails",
        "registryUrl",
        "source",
        "status",
        "taxId",
        "vatId",
        "vatRegistered",
    ];
    let mut keys: Vec<_> = wire
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, expected_keys);
    assert!(wire.get("details").is_none());
    assert_eq!(wire["registryDetails"]["provider"], "RomanianAnaf");
}

#[tokio::test]
async fn shared_dispatch_uses_anaf_and_preserves_negative_observations() {
    for negative in [false, true] {
        let today = jiff::Timestamp::now()
            .in_tz("Europe/Bucharest")
            .expect("bundled Bucharest timezone")
            .date();
        let mut body = fixture();
        body["found"][0]["date_generale"]["data"] = json!(today.to_string());
        if negative {
            body = json!({"cod":200,"message":"SUCCESS","found":[],"notFound":[1234]});
        }
        let (url, task) = server(
            "200 OK",
            "Content-Type: application/json\r\n",
            body.to_string(),
        )
        .await;
        let unused_bg = crate::BgRegistryClient::for_test(
            "http://127.0.0.1:1/unused".into(),
            "http://127.0.0.1:1/unused".into(),
            Default::default(),
        );
        let client = crate::Client::with_test_registries(
            unused_bg,
            RoRegistryClient::for_test(url, Default::default()),
        );
        let id = crate::BusinessId::from(RoBusinessId::parse("001234").unwrap());
        let result = client.lookup_company(id.as_str(), id.country()).await;
        if negative {
            assert!(matches!(
                result,
                Err(LookupError {
                    kind: crate::model::LookupFailure::RegistryNotFound {
                        provider: Provider::RomanianAnaf
                    },
                    ..
                })
            ));
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.id, crate::ErrorId::UnresolvedData);
            assert_eq!(error.details.country.as_deref(), Some(id.country()));
        }
        assert!(task.await.unwrap().starts_with("POST /tva HTTP/1.1"));
    }
}

#[test]
fn address_countries_have_shared_display_casing_and_iso_codes() {
    for (raw, name, code) in [
        ("ROMANIA", "Romania", Some("RO")),
        ("rOmÂnIa", "România", Some("RO")),
        ("ro", "România", Some("RO")),
        ("БЪЛГАРИЯ", "България", Some("BG")),
        ("UNITED KINGDOM", "United Kingdom", Some("GB")),
        ("UNKNOWN COUNTRY", "Unknown Country", None),
    ] {
        let mut value = fixture();
        value["found"][0]["adresa_sediu_social"] = json!({"stara": raw});
        value["found"][0]["adresa_domiciliu_fiscal"]["dtara"] = json!(raw);
        let native = decode(value).unwrap().unwrap();
        for address in [&native.registered_office, &native.fiscal_address] {
            let address = address.as_ref().unwrap();
            assert_eq!(address.country.as_deref(), Some(name));
            assert_eq!(address.country_iso2.as_deref(), code);
        }
        let shared: crate::CompanyLookupResult = native.into();
        crate::test_support::assert_company_contract(&shared);
        for address in [&shared.registered_office, &shared.fiscal_address] {
            let address = address.as_ref().unwrap();
            assert_eq!(address.country.as_deref(), Some(name));
            assert_eq!(address.country_iso2.as_deref(), code);
        }
        assert_eq!(
            shared
                .fiscal_address
                .as_ref()
                .unwrap()
                .full_address_by_registry
                .as_deref(),
            Some("  Strada Exemplu 1\nBucurești  ")
        );
        let wire = serde_json::to_value(shared).unwrap();
        assert_eq!(wire["registeredOffice"]["countryISO2"], json!(code));
        assert_eq!(
            wire["registryDetails"]["data"]["fiscalAddress"]["countryISO2"],
            json!(code)
        );
        assert_eq!(
            wire["registryDetails"]["data"]["registeredOffice"]["country"],
            name
        );
    }
}

#[test]
fn shared_vat_fields_require_matched_anaf_registration_and_preserve_tax_id() {
    for (flag, expected) in [
        (json!(true), Some(true)),
        (json!(false), Some(false)),
        (Value::Null, None),
    ] {
        for raw_id in ["1234", "001234"] {
            let mut source = fixture();
            source["found"][0]["inregistrare_scop_Tva"]["scpTVA"] = flag.clone();
            let native = super::decode::decode(
                &serde_json::to_vec(&source).unwrap(),
                &RoBusinessId::parse(raw_id).unwrap(),
                date(),
            )
            .unwrap()
            .unwrap();
            let shared: crate::CompanyLookupResult = native.into();
            crate::test_support::assert_company_contract(&shared);
            assert_eq!(shared.tax_id, raw_id);
            assert_eq!(shared.vat_registered, expected);
            let vat = if expected == Some(true) {
                Some("RO1234")
            } else {
                None
            };
            assert_eq!(shared.vat_id.as_ref().map(crate::VatId::as_str), vat);
            let wire = serde_json::to_value(shared).unwrap();
            assert_eq!(wire["vatRegistered"], json!(expected));
            assert_eq!(wire["vatId"], json!(vat));
        }
    }
    let mut source = fixture();
    source["found"][0]
        .as_object_mut()
        .unwrap()
        .remove("inregistrare_scop_Tva");
    let shared: crate::CompanyLookupResult = decode(source).unwrap().unwrap().into();
    crate::test_support::assert_company_contract(&shared);
    assert!(shared.vat_registered.is_none());
    assert!(shared.vat_id.is_none());
}

#[test]
fn vat_id_is_not_invented_for_nonstandard_numeric_identity() {
    // Even an inconsistent synthetic positive flag cannot produce a malformed VAT ID.
    for (raw, numeric) in [("00", 0), ("01", 1)] {
        let mut source = fixture();
        source["found"][0]["date_generale"]["cui"] = json!(numeric);
        source["found"][0]["inregistrare_scop_Tva"]["scpTVA"] = json!(true);
        let native = super::decode::decode(
            &serde_json::to_vec(&source).unwrap(),
            &RoBusinessId::parse(raw).unwrap(),
            date(),
        )
        .unwrap()
        .unwrap();
        let shared: crate::CompanyLookupResult = native.into();
        crate::test_support::assert_company_contract(&shared);
        assert_eq!(shared.tax_id, raw);
        assert_eq!(shared.vat_registered, Some(true));
        assert!(shared.vat_id.is_none());
    }
}

#[test]
fn shared_mapping_derives_source_from_anaf_details() {
    let mut native = decode(fixture()).unwrap().unwrap();
    native.source = Provider::Vies;
    let shared: crate::CompanyLookupResult = native.into();
    crate::test_support::assert_company_contract(&shared);
    let wire = serde_json::to_value(shared).unwrap();
    assert_eq!(wire["source"], "RomanianAnaf");
    assert_eq!(wire["source"], wire["registryDetails"]["provider"]);
}

#[test]
fn missing_anaf_address_countries_default_to_romania() {
    for missing in [serde_json::Value::Null, json!(""), json!("   ")] {
        let mut value = fixture();
        value["found"][0]["adresa_sediu_social"] = json!({"stara": missing});
        value["found"][0]["adresa_domiciliu_fiscal"]["dtara"] = missing;
        let native = decode(value).unwrap().unwrap();
        for address in [&native.registered_office, &native.fiscal_address] {
            let address = address.as_ref().unwrap();
            assert_eq!(address.country.as_deref(), Some("Romania"));
            assert_eq!(address.country_iso2.as_deref(), Some("RO"));
        }
        let shared: crate::CompanyLookupResult = native.into();
        crate::test_support::assert_company_contract(&shared);
        for address in [&shared.registered_office, &shared.fiscal_address] {
            let address = address.as_ref().unwrap();
            assert_eq!(address.country.as_deref(), Some("Romania"));
            assert_eq!(address.country_iso2.as_deref(), Some("RO"));
        }
    }
    let mut native = decode(fixture()).unwrap().unwrap();
    native.fiscal_address = None;
    let shared: crate::CompanyLookupResult = native.into();
    crate::test_support::assert_company_contract(&shared);
    assert!(shared.registered_office.is_none());
    let fiscal = shared.fiscal_address.unwrap();
    assert_eq!(fiscal.country.as_deref(), Some("Romania"));
    assert_eq!(fiscal.country_iso2.as_deref(), Some("RO"));
}

#[tokio::test]
async fn raw_registry_errors_use_the_common_object_and_capture_post_body() {
    let (url, task) = server(
        "503 Unavailable",
        "Content-Type: application/json\r\n",
        "{\"message\":\"exact upstream details\"}".into(),
    )
    .await;
    let config = RoRegistryConfig {
        raw_debug: true,
        ..Default::default()
    };
    let error = RoRegistryClient::for_test(url, config)
        .lookup_at("1234", date())
        .await
        .unwrap_err();
    assert_eq!(error.id, crate::ErrorId::ServiceUnavailable);
    assert_eq!(error.details.provider, Some(Provider::RomanianAnaf));
    assert_eq!(error.details.country.as_deref(), Some("RO"));
    let debug = error.details.debug.unwrap();
    assert_eq!(
        debug.exchanges[0].request.content_type.as_deref(),
        Some("application/json")
    );
    assert_eq!(
        serde_json::from_str::<Value>(&debug.exchanges[0].request.body.as_ref().unwrap().data)
            .unwrap(),
        json!([{"cui":1234,"data":"2025-02-14"}])
    );
    assert_eq!(
        debug.exchanges[0].response.as_ref().unwrap().body.data,
        "{\"message\":\"exact upstream details\"}"
    );
    assert!(debug.exchanges[0].response.as_ref().unwrap().complete);
    task.await.unwrap();
}

#[tokio::test]
async fn shared_transport_preserves_miss_and_validation_diagnostics() {
    let body = json!({"found":[], "notFound":[1234]}).to_string();
    let (url, task) = server("200 OK", "Content-Type: application/json\r\n", body.clone()).await;
    let client = RoRegistryClient::for_test(
        url,
        RoRegistryConfig {
            raw_debug: true,
            ..Default::default()
        },
    );
    let error = client.lookup_at("1234", date()).await.unwrap_err();
    assert_eq!(error.id, crate::ErrorId::NotFound);
    assert_eq!(error.details.provider, Some(Provider::RomanianAnaf));
    assert_eq!(error.details.country.as_deref(), Some("RO"));
    assert_eq!(error.details.upstream_http_status, Some(200));
    assert_eq!(
        error.details.response_content_type.as_deref(),
        Some("application/json")
    );
    let exchanges = &error.details.debug.as_ref().unwrap().exchanges;
    assert_eq!(exchanges.len(), 1);
    assert_eq!(exchanges[0].response.as_ref().unwrap().body.data, body);
    task.await.unwrap();
    let error = client.lookup_at("invalid", date()).await.unwrap_err();
    assert_eq!(error.id, crate::ErrorId::InvalidInput);
    assert_eq!(error.details.country.as_deref(), Some("RO"));
    assert!(error.details.upstream_http_status.is_none());
    assert!(error.details.debug.unwrap().exchanges.is_empty());
}

#[test]
fn anaf_company_status_phrases_require_an_exact_label_and_valid_date() {
    for (text, expected) in [
        (
            "INREGISTRAT din data 01.02.2000",
            crate::CompanyStatus::Active,
        ),
        ("RADIERE din data 01.02.2000", crate::CompanyStatus::Closed),
        ("RADIERE din data 99.99.2000", crate::CompanyStatus::Unknown),
        (
            "INREGISTRAT IN SCOPURI DE TVA",
            crate::CompanyStatus::Unknown,
        ),
    ] {
        let mut raw = fixture();
        raw["found"][0]["date_generale"]["stare_inregistrare"] = json!(text);
        let native = decode(raw).unwrap().unwrap();
        assert_eq!(native.status, Some(expected));
        assert_eq!(native.native_registry_status.as_deref(), Some(text));
        assert_eq!(native.tax_status.inactive, Some(true));
    }
}

#[tokio::test]
async fn lifecycle_quality_gate_preserves_source_and_distinguishes_negative_results() {
    for (status, expected) in [
        (Some("INREGISTRAT"), Some(crate::CompanyStatus::Active)),
        (Some("RADIAT"), Some(crate::CompanyStatus::Closed)),
        (Some("FUTURE SOURCE STATE"), None),
        (None, None),
    ] {
        let mut body = fixture();
        if let Some(text) = status {
            body["found"][0]["date_generale"]["stare_inregistrare"] = json!(text);
        }
        let (url, task) = server(
            "200 OK",
            "Content-Type: application/json\r\n",
            body.to_string(),
        )
        .await;
        let config = RoRegistryConfig {
            raw_debug: true,
            ..Default::default()
        };
        let result = RoRegistryClient::for_test(url, config)
            .lookup_at("001234", date())
            .await;
        if let Some(expected) = expected {
            assert_eq!(result.unwrap().status, Some(expected));
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.id, crate::ErrorId::UnresolvedData);
            assert_eq!(error.details.native_registry_status.as_deref(), status);
            assert_eq!(error.details.provider, Some(Provider::RomanianAnaf));
            assert!(!error.is_service_unavailable);
            assert!(error.details.debug.is_some());
            let json = serde_json::to_value(&error).unwrap();
            assert_eq!(json["id"], "unresolved_data");
            assert!(!format!("{error:?}").contains("FUTURE SOURCE STATE"));
        }
        task.await.unwrap();
    }
}

#[test]
fn lifecycle_activity_and_merger_phrases_are_exact() {
    use crate::CompanyStatus::*;
    for (text, expected) in [
        ("SUSPENDARE ACTIVITATE din data 10.01.2025", Inactive),
        ("RELUARE ACTIVITATE din data 01.11.2016", Active),
        (
            "DIZOLVARE FARA LICHIDARE(FUZIUNE) din data 18.02.2015",
            Closed,
        ),
        ("TRANSFER(SOSIRE) din data 22.07.2021", Unknown),
        ("SUSPENDARE ACTIVITATE din data 31.02.2025", Unknown),
        ("RELUARE ACTIVITATE din data 1.1.2025", Unknown),
        ("DIZOLVARE", Unknown),
    ] {
        assert_eq!(mapping::lifecycle(text), expected, "{text}");
    }
}

#[test]
fn documented_company_output_matches_synthetic_source() {
    let mut body = fixture();
    body["found"][0]["date_generale"]["stare_inregistrare"] =
        json!("INREGISTRAT DIN DATA 01.01.2000");
    let mut native = decode(body).unwrap().unwrap();
    native.checked_at = "2025-02-14T12:00:00Z".parse().unwrap();
    crate::test_support::assert_documented_company("ro", native);
}

#[test]
fn fiscal_day_uses_bucharest_midnight_across_dst_changes() {
    for (instant, expected) in [
        ("2026-01-01T21:59:59Z", "2026-01-01"),
        ("2026-01-01T22:00:00Z", "2026-01-02"),
        ("2026-03-29T20:59:59Z", "2026-03-29"),
        ("2026-03-29T21:00:00Z", "2026-03-30"),
        ("2026-10-25T21:59:59Z", "2026-10-25"),
        ("2026-10-25T22:00:00Z", "2026-10-26"),
    ] {
        assert_eq!(
            client::fiscal_date_at(instant.parse().unwrap()).to_string(),
            expected
        );
    }
}
