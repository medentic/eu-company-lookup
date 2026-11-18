//! Private synthetic test helpers; never compiled into the consumer library.
use crate::CompanyLookupResult;

pub(crate) fn assert_company_contract(result: &CompanyLookupResult) {
    let checked = crate::model::company_validation::ensure_resolved(result.clone());
    match result.status {
        None | Some(crate::CompanyStatus::Unknown) => {
            let error = checked.unwrap_err();
            assert_eq!(error.id, crate::ErrorId::UnresolvedData);
            assert_eq!(error.details.provider, Some(result.source));
            assert!(!error.is_service_unavailable);
        }
        Some(_) => assert_eq!(checked.unwrap(), *result),
    }
    for unresolved in [None, Some(crate::CompanyStatus::Unknown)] {
        let mut observation = result.clone();
        observation.status = unresolved;
        let error = crate::model::company_validation::ensure_resolved(observation).unwrap_err();
        assert_eq!(error.id, crate::ErrorId::UnresolvedData);
        assert_eq!(
            error.details.native_registry_status.as_deref(),
            result.registry_details.native_status()
        );
        assert!(!error.is_service_unavailable);
    }
    let wire = serde_json::to_value(result).unwrap();
    assert_eq!(wire["source"], wire["registryDetails"]["provider"]);
    assert!(
        wire["registryDetails"]["data"]
            .as_object()
            .unwrap()
            .contains_key("nativeRegistryStatus")
    );
    let native = &wire["registryDetails"]["data"]["nativeRegistryStatus"];
    assert!(native.is_null() || native.is_string());
    assert!(
        native.is_null() || !wire["status"].is_null(),
        "a supplied source status must have a common known/unknown status"
    );
    let mut keys: Vec<_> = wire
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
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
            "vatRegistered"
        ]
    );
}

pub(crate) mod http;

pub(crate) mod company_validation;

pub(crate) mod retained_registry;

/// Compare maintained synthetic JSON with the current projection and serializer.
pub(crate) fn assert_documented_company(country: &str, actual: impl Into<CompanyLookupResult>) {
    let expected = documentation::example(country, 0);
    let actual = actual.into();
    assert_company_contract(&actual);
    assert!(crate::model::company_validation::ensure_resolved(actual.clone()).is_ok());
    assert_eq!(
        serde_json::to_value(actual).unwrap(),
        expected,
        "{country} documented output drifted"
    );
}

pub(crate) mod documentation;
