use super::*;
use crate::{LookupError, Provider, RegistryError, RetryAfter};
use chrono::NaiveDate;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2025, 2, 14).unwrap()
}
fn fixture() -> Value {
    json!({"cod":200,"message":"SUCCESS","found":[{
        "date_generale":{"cui":1234,"data":"2025-02-14","denumire":"EXEMPLU SRL","adresa":"  Strada Exemplu 1\nBucurești  ","nrRegCom":"J00/1/2000","statusRO_e_Factura":false},
        "inregistrare_scop_Tva":{"scpTVA":false},
        "stare_inactiv":{"statusInactivi":true},
        "adresa_domiciliu_fiscal":{"dcod_Postal":"012345","ddenumire_Localitate":"București"}
    }],"notFound":[]})
}
fn decode(value: Value) -> Result<Option<RoCompanyLookupResult>, RegistryError> {
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
) -> (String, tokio::task::JoinHandle<String>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/tva", listener.local_addr().unwrap());
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}",
        body.len()
    );
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        loop {
            let mut buffer = [0; 4096];
            let n = socket.read(&mut buffer).await.unwrap();
            if n == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..n]);
            if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                let header = String::from_utf8_lossy(&request[..end]);
                let size = header
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|n| n.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                if request.len() >= end + 4 + size {
                    break;
                }
            }
        }
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
        client
            .lookup_at("001234", date())
            .await
            .unwrap()
            .tax_id
            .as_str(),
        "001234"
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
                LookupError::RegistryNotFound {
                    provider: Provider::RomanianAnaf,
                },
            ) => {}
            (
                "http",
                LookupError::Registry {
                    error: RegistryError::Http { .. },
                    ..
                },
            ) => {}
            (
                "access",
                LookupError::Registry {
                    error: RegistryError::AccessDenied,
                    ..
                },
            ) => {}
            (
                "schema",
                LookupError::Registry {
                    error: RegistryError::UnexpectedResponse,
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
        Err(LookupError::Registry {
            error: RegistryError::RateLimited {
                retry_after: Some(RetryAfter::Seconds(120))
            },
            ..
        })
    ));
    task.await.unwrap();
    assert!(matches!(
        client.clone().lookup_at("1234", date()).await,
        Err(LookupError::Registry {
            error: RegistryError::CoolingDown { .. },
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
        Err(LookupError::Registry {
            error: RegistryError::UnexpectedResponse,
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
        ..Default::default()
    };
    assert!(matches!(
        RoRegistryClient::for_test(url, config)
            .lookup_at("1234", date())
            .await,
        Err(LookupError::Registry {
            error: RegistryError::ResponseTooLarge,
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
    client.lookup_at("1234", date()).await.unwrap();
    task.await.unwrap();
    assert!(matches!(
        client.lookup_at("1234", date()).await,
        Err(LookupError::Busy)
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
        Err(LookupError::Busy)
    ));
    assert!(task.await.unwrap().unwrap_err().is_service_unavailable());
    assert!(matches!(
        client.lookup_at("1234", date()).await,
        Err(LookupError::Registry {
            error: RegistryError::CoolingDown { .. },
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
    for raw in ["NECUNOSCUT", "INREGISTRAT", "RADIAT"] {
        let mut value = fixture();
        value["found"][0]["date_generale"]["stare_inregistrare"] = json!(raw);
        let record = decode(value).unwrap().unwrap();
        assert_eq!(record.status, Some(crate::CompanyStatus::Unknown));
        assert_eq!(record.registration_status.as_deref(), Some(raw));
        assert_eq!(record.tax_status.inactive, Some(true));
        assert_eq!(serde_json::to_value(record).unwrap()["status"], "unknown");
    }
}

#[test]
fn native_registry_status_keeps_exact_anaf_text_and_missing_value() {
    let missing = decode(fixture()).unwrap().unwrap();
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
    assert_eq!(shared.identifier.as_str(), "001234");
    assert_eq!(shared.identifier.country(), "RO");
    assert!(shared.registered_office.is_none());
    assert!(shared.status.is_none()); // Fiscal inactivity is not legal closure.
    let fiscal = shared.fiscal_address.as_ref().unwrap();
    assert_eq!(fiscal.full_address_by_registry, native.full_fiscal_address);
    assert_eq!(fiscal.postcode.as_deref(), Some("012345"));
    assert!(fiscal.country_iso2.is_none()); // Never inferred from identifier country.
    let crate::CompanyDetails::RomanianAnaf(details) = &shared.details else {
        panic!("wrong details")
    };
    assert_eq!(details.fiscal_address, native.fiscal_address);
    assert_eq!(details.tax_status, native.tax_status);
    let wire = serde_json::to_value(shared).unwrap();
    assert!(wire.get("valid").is_none());
    assert_eq!(wire["details"]["provider"], "RomanianAnaf");
}

#[tokio::test]
async fn shared_dispatch_uses_anaf_and_preserves_negative_observations() {
    for negative in [false, true] {
        let today = chrono::Utc::now()
            .with_timezone(&chrono_tz::Europe::Bucharest)
            .date_naive();
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
        let result = client.lookup_company(&id).await;
        if negative {
            assert!(matches!(
                result,
                Err(LookupError::RegistryNotFound {
                    provider: Provider::RomanianAnaf
                })
            ));
        } else {
            let result = result.unwrap();
            assert_eq!(result.identifier, id);
            assert!(result.status.is_none());
        }
        assert!(task.await.unwrap().starts_with("POST /tva HTTP/1.1"));
    }
}
