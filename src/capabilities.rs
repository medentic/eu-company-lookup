use crate::{
    ChecksumSupport, IdentifierScheme, JurisdictionCapabilities, LanguageSupport, Operation,
    Provider, ProviderCapabilities,
};

/// Discover compiled provider support without constructing clients or making requests.
/// This catalog is independent of a particular client's enabled adapters and live health.
pub fn capabilities() -> Vec<JurisdictionCapabilities> {
    let vies = crate::vies::capabilities::catalog();
    crate::locations::Country::ALL
        .iter()
        .map(|country| {
            let code = country.iso2();
            let mut entry = vies
                .iter()
                .find(|v| v.country_code == code)
                .cloned()
                .unwrap_or(JurisdictionCapabilities {
                    country_code: code,
                    subdivision_code: None,
                    vat_prefix: None,
                    providers: vec![],
                });
            match code {
                "BG" => {
                    for provider in [Provider::BulgarianCommercialRegister, Provider::Bulstat] {
                        entry.providers.push(registry(
                            provider,
                            IdentifierScheme::BgBusinessId,
                            Operation::ValidateBgBusinessIdSyntax,
                            Operation::LookupBgCompany,
                            LanguageSupport {
                                available: vec!["bg"],
                                default: Some("bg"),
                                selectable: false,
                            },
                            true,
                        ));
                    }
                }
                "RO" => entry.providers.push(registry(
                    Provider::RomanianAnaf,
                    IdentifierScheme::RoBusinessId,
                    Operation::ValidateRoBusinessIdSyntax,
                    Operation::LookupRoCompany,
                    LanguageSupport {
                        available: vec!["ro"],
                        default: None,
                        selectable: false,
                    },
                    false,
                )),
                _ => {}
            }
            entry
        })
        .collect()
}

fn registry(
    provider: Provider,
    identifier_scheme: IdentifierScheme,
    validation: Operation,
    lookup: Operation,
    languages: LanguageSupport,
    address_splitting: bool,
) -> ProviderCapabilities {
    ProviderCapabilities {
        provider,
        operations: [validation, lookup, Operation::LookupCompany]
            .into_iter()
            .chain((provider == Provider::RomanianAnaf).then_some(Operation::LookupRoCompanyAt))
            .map(|op| crate::OperationCapabilities::new(op, identifier_scheme))
            .collect(),
        checksum: ChecksumSupport::NotImplemented,
        languages,
        requires_credentials: false,
        optional_owner_details: true,
        address_splitting,
    }
}
