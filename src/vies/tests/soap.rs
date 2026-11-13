use super::super::{VatId, soap};
use crate::{LimitScope, LookupError};

pub(super) const VALID: &str = include_str!("../fixtures/valid.xml");
pub(super) fn fault(code: &str) -> String {
    format!(
        r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><s:Fault><faultcode>s:Server</faultcode><faultstring>{code}</faultstring></s:Fault></s:Body></s:Envelope>"#
    )
}
fn decode(xml: &str) -> Result<soap::Response, LookupError> {
    soap::decode(xml, &VatId::parse("BG123456789").unwrap())
}

#[test]
fn identity_is_preserved_without_translation_or_address_inference() {
    let result = decode(VALID).unwrap();
    assert!(result.valid);
    assert_eq!(result.request_date, "2026-11-13+02:00");
    let owner = result.owner.unwrap();
    assert_eq!(owner.name.as_deref(), Some(" Синтетична & Δοκιμή SRL "));
    assert_eq!(
        owner.address.as_deref(),
        Some("Примерна улица 1\nСофия 1000")
    );
}

#[test]
fn booleans_and_owner_absence_have_explicit_semantics() {
    for value in ["true", "1", " true "] {
        assert!(
            decode(&VALID.replace("<valid>true</valid>", &format!("<valid>{value}</valid>")))
                .unwrap()
                .valid
        );
    }
    for value in ["false", "0"] {
        let result =
            decode(&VALID.replace("<valid>true</valid>", &format!("<valid>{value}</valid>")))
                .unwrap();
        assert!(!result.valid);
        assert!(result.owner.is_none());
    }
    let start = VALID.find("<name>").unwrap();
    let end = VALID.find("</address>").unwrap() + "</address>".len();
    for fields in [
        "",
        "<name/><address/>",
        "<name> --- </name><address>\n--\t</address>",
        r#"<name xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:nil="true"/><address xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:nil="1"/>"#,
    ] {
        let mut xml = VALID.to_string();
        xml.replace_range(start..end, fields);
        assert!(decode(&xml).unwrap().owner.is_none());
    }
}

#[test]
fn malformed_ambiguous_and_wrong_namespace_responses_cannot_validate() {
    let malformed = [
        "<html>error</html>".into(),
        VALID.replace("<valid>true</valid>", "<valid>yes</valid>"),
        VALID.replace("<valid>true</valid>", "<valid>true</valid><valid>false</valid>"),
        VALID.replace("<valid>true</valid>", ""),
        VALID.replace("<valid>true</valid>", "<valid><nested>true</nested></valid>"),
        VALID.replace("<valid>true</valid>", r#"<valid xmlns="wrong">true</valid>"#),
        VALID.replace("<valid>true</valid>", r#"<valid xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:nil="true">true</valid>"#),
        VALID.replace("<name>", r#"<name xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:nil="yes">"#),
        VALID.replace("</s:Body>", "</s:Body><s:Body/>"),
        VALID.replace("</s:Body>", "<s:Fault><faultstring>TIMEOUT</faultstring></s:Fault></s:Body>"),
        VALID.replace("</s:Body>", "<extra/></s:Body>"),
        VALID.replace("<name>", "<unexpected/ ><name>"),
        VALID.replace("http://schemas.xmlsoap.org/soap/envelope/", "wrong"),
        format!("<!DOCTYPE s:Envelope [<!ENTITY injected 'test'>]>{VALID}"),
        fault("<nested/>"),
    ];
    for xml in malformed {
        assert!(
            matches!(decode(&xml), Err(LookupError::MalformedResponse)),
            "{xml}"
        );
    }
    for name in ["countryCode", "vatNumber", "requestDate", "name", "address"] {
        let xml = VALID.replace(
            "</checkVatResponse>",
            &format!("<{name}>duplicate</{name}></checkVatResponse>"),
        );
        assert!(matches!(decode(&xml), Err(LookupError::MalformedResponse)));
    }
}

#[test]
fn exact_requested_identity_is_required_even_for_negative_results() {
    for xml in [
        VALID.replace("BG", "EL"),
        VALID.replace("123456789", "987654321"),
        VALID.replace("123456789", " 123456789 "),
    ] {
        assert!(matches!(decode(&xml), Err(LookupError::MismatchedResponse)));
        assert!(matches!(
            decode(&xml.replace("<valid>true", "<valid>false")),
            Err(LookupError::MismatchedResponse)
        ));
    }
}

#[test]
fn source_dates_are_real_calendar_dates_with_valid_optional_offsets() {
    for date in [
        "2024-02-29",
        "2026-11-13Z",
        "2026-11-13+14:00",
        "2026-11-13-03:30",
    ] {
        assert!(decode(&VALID.replace("2026-11-13+02:00", date)).is_ok());
    }
    for date in [
        "",
        "2025-02-29",
        "2026-13-01",
        "0000-01-01",
        "2026-1-01",
        "2026-11-13+14:01",
        "2026-11-13+01:60",
        "2026-11-13+15:00",
        "2026-11-13T00:00:00Z",
        "ééééééééééééé",
    ] {
        assert!(
            matches!(
                decode(&VALID.replace("2026-11-13+02:00", date)),
                Err(LookupError::MalformedResponse)
            ),
            "{date}"
        );
    }
}

#[test]
fn every_documented_fault_is_a_failure_not_false() {
    assert!(matches!(
        decode(&fault("INVALID_INPUT")),
        Err(LookupError::ProviderRejectedInput)
    ));
    assert!(matches!(
        decode(&fault("SERVICE_UNAVAILABLE")),
        Err(LookupError::ServiceUnavailable { .. })
    ));
    assert!(matches!(
        decode(&fault("MS_UNAVAILABLE")),
        Err(LookupError::MemberStateUnavailable { .. })
    ));
    assert!(matches!(
        decode(&fault("TIMEOUT")),
        Err(LookupError::ProviderTimeout { .. })
    ));
    assert!(matches!(
        decode(&fault("GLOBAL_MAX_CONCURRENT_REQ")),
        Err(LookupError::RateLimited {
            scope: LimitScope::Global,
            ..
        })
    ));
    assert!(matches!(
        decode(&fault("MS_MAX_CONCURRENT_REQ")),
        Err(LookupError::RateLimited {
            scope: LimitScope::MemberState,
            ..
        })
    ));
    let unknown = decode(&fault("private arbitrary upstream message"))
        .err()
        .unwrap();
    assert!(matches!(unknown, LookupError::UnknownSoapFault));
    assert!(!format!("{unknown:?} {unknown}").contains("private"));
}

#[test]
fn request_is_the_documented_soap_operation() {
    let request = soap::request(&VatId::parse("IE1+12345B").unwrap());
    assert!(request.contains("<countryCode>IE</countryCode><vatNumber>1+12345B</vatNumber>"));
    assert!(!request.contains("language"));
    assert!(roxmltree::Document::parse(&request).is_ok());
}
