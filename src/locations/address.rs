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
    /// Source country name with recognized Bulgarian names normalized for display.
    /// Not inferred from the registry jurisdiction; original casing stays in source text.
    pub country: Option<String>,
    /// Address country code, only when the source country is recognized.
    #[serde(rename = "countryISO2")]
    pub country_iso2: Option<String>,
}
