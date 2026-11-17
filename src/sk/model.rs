/// Selected native public accounting-entity observations.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct SkCompanyDetails {
    /// Original source `id` field.
    #[serde(rename = "id")]
    pub id: u64,
    /// Original source `ico` field.
    #[serde(rename = "ico")]
    pub ico: String,
    /// Original source `nazovUJ` field.
    #[serde(rename = "nazovUJ")]
    pub nazov_uj: String,
    /// Original source `dic` field.
    #[serde(rename = "dic")]
    pub dic: Option<String>,
    /// Original source `ulica` field.
    #[serde(rename = "ulica")]
    pub ulica: Option<String>,
    /// Original source `mesto` field.
    #[serde(rename = "mesto")]
    pub mesto: Option<String>,
    /// Original source `psc` field.
    #[serde(rename = "psc")]
    pub psc: Option<String>,
    /// Original source `pravnaForma` field.
    #[serde(rename = "pravnaForma")]
    pub pravna_forma: Option<String>,
    /// Original source `datumZalozenia` field.
    #[serde(rename = "datumZalozenia")]
    pub datum_zalozenia: Option<String>,
    /// Original source `datumZrusenia` field.
    #[serde(rename = "datumZrusenia")]
    pub datum_zrusenia: Option<String>,
    /// Original source `datumPoslednejUpravy` field.
    #[serde(rename = "datumPoslednejUpravy")]
    pub datum_poslednej_upravy: Option<String>,
    /// Original source `zdrojDat` field.
    #[serde(rename = "zdrojDat")]
    pub zdroj_dat: Option<String>,
    /// Original source `sidlo` field.
    #[serde(rename = "sidlo")]
    pub sidlo: Option<String>,
    /// Original source `kraj` field.
    #[serde(rename = "kraj")]
    pub kraj: Option<String>,
    /// Original source `okres` field.
    #[serde(rename = "okres")]
    pub okres: Option<String>,
    /// Original source `skNace` field.
    #[serde(rename = "skNace")]
    pub sk_nace: Option<String>,
    /// Original source `velkostOrganizacie` field.
    #[serde(rename = "velkostOrganizacie")]
    pub velkost_organizacie: Option<String>,
    /// Original source `druhVlastnictva` field.
    #[serde(rename = "druhVlastnictva")]
    pub druh_vlastnictva: Option<String>,
    /// Original source `stav` field.
    #[serde(rename = "stav")]
    pub stav: Option<String>,
}
/// Matched Slovak accounting entity.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SkCompanyLookupResult {
    /// Requested full ICO.
    pub tax_id: super::SkBusinessId,
    /// Original source name.
    pub company_name: String,
    /// Native source observations.
    pub details: SkCompanyDetails,
}
