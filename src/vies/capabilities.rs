use super::identifiers::FORMATS;
use crate::{
    ChecksumSupport, IdentifierScheme, JurisdictionCapabilities, LanguageSupport, Operation,
    Provider, ProviderCapabilities,
};

pub(crate) fn catalog() -> Vec<JurisdictionCapabilities> {
    FORMATS
        .iter()
        .map(|(prefix, _)| JurisdictionCapabilities {
            country_code: match *prefix {
                "EL" => "GR",
                "XI" => "GB",
                other => other,
            },
            subdivision_code: (*prefix == "XI").then_some("GB-NIR"),
            vat_prefix: Some(prefix),
            providers: vec![ProviderCapabilities {
                provider: Provider::Vies,
                operations: vec![
                    Operation::ValidateVatSyntax,
                    Operation::LookupVat,
                    Operation::IsVatValid,
                ]
                .into_iter()
                .map(|op| crate::OperationCapabilities::new(op, IdentifierScheme::Vat))
                .collect(),
                checksum: ChecksumSupport::NotImplemented,
                languages: LanguageSupport {
                    available: vec![],
                    default: None,
                    selectable: false,
                },
                requires_credentials: false,
                optional_owner_details: true,
                address_splitting: super::address::supports(prefix),
            }],
        })
        .collect()
}
