use serde::Serialize;

/// Provider-independent structured address. Missing components remain unknown.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Address {
    /// Primary delivery line, when recognizable.
    pub address: Option<String>,
    /// Remaining delivery lines.
    pub address2: Option<String>,
    /// Source settlement name, without the Bulgarian town/village marker.
    pub city: Option<String>,
    /// Source postcode; leading zeroes are retained.
    pub postcode: Option<String>,
    /// Source administrative region.
    pub region: Option<String>,
    /// Source municipality.
    pub municipality: Option<String>,
    /// Title-case country display name, retaining its language and diacritics.
    /// Not inferred from the registry jurisdiction; original casing stays in source text.
    pub country: Option<String>,
    /// Uppercase ISO 3166-1 alpha-2 country code, when recognized.
    #[serde(rename = "countryISO2")]
    pub country_iso2: Option<String>,
}

impl Address {
    pub(crate) fn normalize_country(&mut self) {
        if self.country_iso2.is_none() {
            self.country_iso2 = self
                .country
                .as_deref()
                .and_then(super::Country::from_name)
                .map(|country| country.iso2().to_owned());
        }
        if let Some(code) = &mut self.country_iso2 {
            *code = code.to_ascii_uppercase();
        }
        if let Some(name) = &mut self.country {
            *name = super::normalize::country_name(name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn country_casing_preserves_unicode_and_does_not_guess_missing_values() {
        let mut address = Address {
            country: Some("BELGIË / BELGIQUE / BELGIEN".into()),
            country_iso2: Some("be".into()),
            ..Default::default()
        };
        address.normalize_country();
        assert_eq!(
            address.country.as_deref(),
            Some("België / Belgique / Belgien")
        );
        assert_eq!(address.country_iso2.as_deref(), Some("BE"));
        let mut missing = Address::default();
        missing.normalize_country();
        assert!(missing.country.is_none());
        assert!(missing.country_iso2.is_none());
    }
}
