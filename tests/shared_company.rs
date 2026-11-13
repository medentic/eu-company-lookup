//! Shared business lookup contracts; all tests are offline.
use eu_company_lookup::{
    BgBusinessId, BusinessId, Client, CompanyLookupResult, IdentifierScheme, LookupError,
    Operation, RoBusinessId,
};
use serde_json::json;

#[test]
fn identifiers_validate_country_scheme_and_wire_input() {
    for (country, scheme, raw, expected) in [
        (
            "bg",
            IdentifierScheme::BgBusinessId,
            "000 1234567890",
            "0001234567890",
        ),
        ("RO", IdentifierScheme::RoBusinessId, "00 1234", "001234"),
    ] {
        let id = BusinessId::parse(country, scheme, raw).unwrap();
        assert_eq!(id.as_str(), expected);
        assert_eq!(id.country(), country.to_uppercase());
        assert_eq!(id.scheme(), scheme);
        let wire = serde_json::to_value(&id).unwrap();
        assert_eq!(serde_json::from_value::<BusinessId>(wire).unwrap(), id);
    }
    for wire in [
        json!({"country":"BG","scheme":"RoBusinessId","value":"1234"}),
        json!({"country":"RO","scheme":"BgBusinessId","value":"123456789"}),
        json!({"country":"RO","scheme":"Vat","value":"RO1234"}),
        json!({"country":"BG","scheme":"BgBusinessId","value":"1234567890"}),
        json!({"country":"NO","scheme":"RoBusinessId","value":"1234"}),
        json!({"country":"RO","scheme":"RoBusinessId","value":1234}),
    ] {
        assert!(serde_json::from_value::<BusinessId>(wire).is_err());
    }
}

#[test]
fn one_lookup_signature_accepts_both_national_types() {
    let client = Client::new().unwrap();
    fn shared(
        _: impl std::future::Future<Output = Result<CompanyLookupResult, LookupError>> + Send,
    ) {
    }
    for id in [
        BusinessId::from(BgBusinessId::parse("123456789").unwrap()),
        BusinessId::from(RoBusinessId::parse("001234").unwrap()),
    ] {
        shared(client.lookup_company(&id)); // Not polled; no network calls.
    }
}

#[test]
fn non_vies_eea_countries_are_discoverable_without_claiming_lookup_support() {
    let catalog = Client::new().unwrap().capabilities();
    for code in ["NO", "IS", "LI"] {
        let entry = catalog.iter().find(|c| c.country_code == code).unwrap();
        assert!(entry.vat_prefix.is_none());
        assert!(entry.providers.is_empty());
    }
    for code in ["BG", "RO"] {
        let entry = catalog.iter().find(|c| c.country_code == code).unwrap();
        assert!(
            entry
                .providers
                .iter()
                .any(|p| p.operations.contains(&Operation::LookupCompany))
        );
        assert!(entry.providers.iter().all(|p| !p.languages.selectable));
    }
}
