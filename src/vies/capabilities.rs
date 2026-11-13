use super::identifiers::FORMATS;
use crate::{
    ChecksumSupport, IdentifierScheme, JurisdictionCapabilities, LanguageSupport, Operation,
    Provider, ProviderCapabilities,
};

pub(super) fn catalog() -> Vec<JurisdictionCapabilities> {
    FORMATS
        .iter()
        .map(|(prefix, _)| JurisdictionCapabilities {
            country_code: match *prefix {
                "EL" => "GR",
                "XI" => "GB",
                other => other,
            },
            subdivision_code: (*prefix == "XI").then_some("GB-NIR"),
            vat_prefix: prefix,
            providers: vec![ProviderCapabilities {
                provider: Provider::Vies,
                operations: vec![
                    Operation::ValidateVatSyntax,
                    Operation::LookupVat,
                    Operation::IsVatValid,
                ],
                identifier_scheme: IdentifierScheme::Vat,
                checksum: ChecksumSupport::NotImplemented,
                languages: LanguageSupport::NotSelectable,
                requires_credentials: false,
                optional_owner_details: true,
            }],
        })
        .collect()
}
