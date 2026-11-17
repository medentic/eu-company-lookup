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
            if !crate::Client::SUPPORTED_COMPANY_COUNTRIES.contains(&code) {
                return entry;
            }
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
                "CZ" => entry.providers.push(registry(
                    Provider::CzechAres,
                    IdentifierScheme::CzBusinessId,
                    Operation::ValidateCzBusinessIdSyntax,
                    Operation::LookupCzCompany,
                    LanguageSupport {
                        available: vec!["cs"],
                        default: None,
                        selectable: false,
                    },
                    false,
                )),
                "AT" => entry.providers.push(registry(
                    Provider::AustrianFirmenbuch,
                    IdentifierScheme::AtBusinessId,
                    Operation::ValidateAtBusinessIdSyntax,
                    Operation::LookupAtCompany,
                    LanguageSupport {
                        available: vec!["de"],
                        default: None,
                        selectable: false,
                    },
                    false,
                )),
                "BE" => entry.providers.push(registry(
                    Provider::BelgianNbb,
                    IdentifierScheme::BeBusinessId,
                    Operation::ValidateBeBusinessIdSyntax,
                    Operation::LookupBeCompany,
                    LanguageSupport {
                        available: vec!["en"],
                        default: Some("en"),
                        selectable: false,
                    },
                    false,
                )),
                "FI" => entry.providers.push(registry(
                    Provider::FinnishPrh,
                    IdentifierScheme::FiBusinessId,
                    Operation::ValidateFiBusinessIdSyntax,
                    Operation::LookupFiCompany,
                    LanguageSupport {
                        available: vec!["fi", "sv", "en"],
                        default: None,
                        selectable: false,
                    },
                    false,
                )),
                "FR" => entry.providers.push(registry(
                    Provider::FrenchRechercheEntreprises,
                    IdentifierScheme::FrBusinessId,
                    Operation::ValidateFrBusinessIdSyntax,
                    Operation::LookupFrCompany,
                    LanguageSupport {
                        available: vec!["fr"],
                        default: Some("fr"),
                        selectable: false,
                    },
                    false,
                )),
                "IS" => entry.providers.push(registry(
                    Provider::IcelandicSkatturinn,
                    IdentifierScheme::IsBusinessId,
                    Operation::ValidateIsBusinessIdSyntax,
                    Operation::LookupIsCompany,
                    LanguageSupport {
                        available: vec!["is"],
                        default: Some("is"),
                        selectable: false,
                    },
                    false,
                )),
                "IE" => entry.providers.push(registry(
                    Provider::IrishCro,
                    IdentifierScheme::IeBusinessId,
                    Operation::ValidateIeBusinessIdSyntax,
                    Operation::LookupIeCompany,
                    LanguageSupport {
                        available: vec!["en"],
                        default: Some("en"),
                        selectable: false,
                    },
                    false,
                )),
                "LT" => entry.providers.push(registry(
                    Provider::LithuanianJar,
                    IdentifierScheme::LtBusinessId,
                    Operation::ValidateLtBusinessIdSyntax,
                    Operation::LookupLtCompany,
                    LanguageSupport {
                        available: vec!["lt"],
                        default: Some("lt"),
                        selectable: false,
                    },
                    false,
                )),
                "LV" => entry.providers.push(registry(
                    Provider::LatvianOpenData,
                    IdentifierScheme::LvBusinessId,
                    Operation::ValidateLvBusinessIdSyntax,
                    Operation::LookupLvCompany,
                    LanguageSupport {
                        available: vec!["lv"],
                        default: Some("lv"),
                        selectable: false,
                    },
                    false,
                )),
                "SK" => entry.providers.push(registry(
                    Provider::SlovakRuz,
                    IdentifierScheme::SkBusinessId,
                    Operation::ValidateSkBusinessIdSyntax,
                    Operation::LookupSkCompany,
                    LanguageSupport {
                        available: vec!["sk"],
                        default: Some("sk"),
                        selectable: false,
                    },
                    false,
                )),
                "PL" => entry.providers.push(registry(
                    Provider::PolishKrs,
                    IdentifierScheme::PlBusinessId,
                    Operation::ValidatePlBusinessIdSyntax,
                    Operation::LookupPlCompany,
                    LanguageSupport {
                        available: vec!["pl"],
                        default: Some("pl"),
                        selectable: false,
                    },
                    false,
                )),
                "NO" => entry.providers.push(registry(
                    Provider::NorwegianBrreg,
                    IdentifierScheme::NoBusinessId,
                    Operation::ValidateNoBusinessIdSyntax,
                    Operation::LookupNoCompany,
                    LanguageSupport {
                        available: vec!["nb"],
                        default: Some("nb"),
                        selectable: false,
                    },
                    false,
                )),
                "EE" => entry.providers.push(registry(
                    Provider::EstonianRik,
                    IdentifierScheme::EeBusinessId,
                    Operation::ValidateEeBusinessIdSyntax,
                    Operation::LookupEeCompany,
                    LanguageSupport {
                        available: vec!["en"],
                        default: Some("en"),
                        selectable: false,
                    },
                    false,
                )),
                "GR" => entry.providers.push(registry(
                    Provider::GreekGemi,
                    IdentifierScheme::GrBusinessId,
                    Operation::ValidateGrBusinessIdSyntax,
                    Operation::LookupGrCompany,
                    LanguageSupport {
                        available: vec!["el"],
                        default: Some("el"),
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
