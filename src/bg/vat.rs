//! Conservative BG-only VAT enrichment; registry identity is never rewritten.
use crate::{CompanyDetails, CompanyLookupResult, LookupError, VatId, VatLookupResult};

/// Bulgarian company orchestration owns VIES as a required dependency.
#[derive(Clone, Debug)]
pub(crate) struct BgCompanyClient {
    registry: super::BgRegistryClient,
    vies: crate::vies::ViesClient,
}

impl BgCompanyClient {
    pub(crate) fn new(registry: super::BgRegistryClient, vies: crate::vies::ViesClient) -> Self {
        Self { registry, vies }
    }

    pub(crate) async fn lookup(&self, value: &str) -> Result<CompanyLookupResult, LookupError> {
        let mut company: CompanyLookupResult = self.registry.lookup(value).await?.into();
        if let Some(candidate) = vat_candidate(&company) {
            let verification = self.vies.lookup_vat(candidate.as_str()).await?;
            apply_vat_verification(&mut company, verification);
        }
        Ok(company)
    }

    #[cfg(test)]
    pub(crate) fn set_test_vies(&mut self, vies: crate::vies::ViesClient) {
        self.vies = vies;
    }
}

pub(super) fn vat_candidate(company: &CompanyLookupResult) -> Option<VatId> {
    // No truncation or parent-company inference for thirteen-digit branches.
    if company.country != "BG"
        || company.tax_id.len() != 9
        || !company.tax_id.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    VatId::parse(&format!("BG{}", company.tax_id)).ok()
}

fn normalized_name(name: &str) -> String {
    // Keep word boundaries and legal-form tokens. No transliteration, suffix
    // removal, alias inference or fuzzy matching. Source spelling remains intact.
    name.chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .map(str::to_uppercase)
        .collect::<Vec<_>>()
        .join(" ")
}

fn apply_vat_verification(company: &mut CompanyLookupResult, observation: VatLookupResult) {
    let expected = vat_candidate(company);
    let name = normalized_name(&company.company_name);
    let matched = observation.valid
        && expected.as_ref() == Some(&observation.vat_id)
        && !name.is_empty()
        && observation
            .company
            .as_deref()
            .is_some_and(|v| normalized_name(v) == name);
    if matched {
        company.vat_registered = Some(true);
        company.vat_id = Some(observation.vat_id.clone());
    }
    match &mut company.registry_details {
        CompanyDetails::BulgarianCommercialRegister {
            vat_verification, ..
        }
        | CompanyDetails::Bulstat {
            vat_verification, ..
        } => {
            *vat_verification = Some(Box::new(observation));
        }
        CompanyDetails::RomanianAnaf { .. }
        | CompanyDetails::CzechAres { .. }
        | CompanyDetails::AustrianFirmenbuch { .. }
        | CompanyDetails::EstonianRik { .. }
        | CompanyDetails::NorwegianBrreg { .. }
        | CompanyDetails::PolishKrs { .. }
        | CompanyDetails::SlovakRuz { .. }
        | CompanyDetails::LatvianOpenData { .. }
        | CompanyDetails::LithuanianJar { .. }
        | CompanyDetails::IrishCro { .. }
        | CompanyDetails::IcelandicSkatturinn { .. }
        | CompanyDetails::FrenchRechercheEntreprises { .. }
        | CompanyDetails::FinnishPrh { .. }
        | CompanyDetails::BelgianNbb { .. }
        | CompanyDetails::GreekGemi { .. } => {
            unreachable!("BG enrichment only")
        }
    }
}
