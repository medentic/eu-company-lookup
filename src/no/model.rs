/// Native code and source description.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct NoClassification {
    /// Original source `kode` value.
    #[serde(rename = "kode")]
    pub kode: String,
    /// Original source `beskrivelse` value.
    #[serde(rename = "beskrivelse")]
    pub beskrivelse: Option<String>,
}
/// Native address with its source role retained.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct NoRegistryAddress {
    /// Original source `land` value.
    #[serde(rename = "land")]
    pub land: Option<String>,
    /// Original source `landkode` value.
    #[serde(rename = "landkode")]
    pub landkode: Option<String>,
    /// Original source `postnummer` value.
    #[serde(rename = "postnummer")]
    pub postnummer: Option<String>,
    /// Original source `poststed` value.
    #[serde(rename = "poststed")]
    pub poststed: Option<String>,
    /// Original source `kommune` value.
    #[serde(rename = "kommune")]
    pub kommune: Option<String>,
    /// Original source `kommunenummer` value.
    #[serde(rename = "kommunenummer")]
    pub kommunenummer: Option<String>,
    /// Original source `adresse` value.
    #[serde(rename = "adresse")]
    pub adresse: Option<Vec<String>>,
}
/// Selected native entity observations; postal and business roles remain distinct.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct NoCompanyDetails {
    /// Original source `organisasjonsnummer` value.
    #[serde(rename = "organisasjonsnummer")]
    pub organisasjonsnummer: String,
    /// Original source `navn` value.
    #[serde(rename = "navn")]
    pub navn: String,
    /// Original source `organisasjonsform` value.
    #[serde(rename = "organisasjonsform")]
    pub organisasjonsform: Option<NoClassification>,
    /// Original source `forretningsadresse` value.
    #[serde(rename = "forretningsadresse")]
    pub forretningsadresse: Option<NoRegistryAddress>,
    /// Original source `postadresse` value.
    #[serde(rename = "postadresse")]
    pub postadresse: Option<NoRegistryAddress>,
    /// Original source `registrertIMvaregisteret` value.
    #[serde(rename = "registrertIMvaregisteret")]
    pub registrert_i_mvaregisteret: Option<bool>,
    /// Original source `registrertIForetaksregisteret` value.
    #[serde(rename = "registrertIForetaksregisteret")]
    pub registrert_i_foretaksregisteret: Option<bool>,
    /// Original source `konkurs` value.
    #[serde(rename = "konkurs")]
    pub konkurs: Option<bool>,
    /// Original source `underAvvikling` value.
    #[serde(rename = "underAvvikling")]
    pub under_avvikling: Option<bool>,
    /// Original source `underTvangsavviklingEllerTvangsopplosning` value.
    #[serde(rename = "underTvangsavviklingEllerTvangsopplosning")]
    pub under_tvangsavvikling_eller_tvangsopplosning: Option<bool>,
    /// Original source `stiftelsesdato` value.
    #[serde(rename = "stiftelsesdato")]
    pub stiftelsesdato: Option<String>,
    /// Original source `slettedato` value.
    #[serde(rename = "slettedato")]
    pub slettedato: Option<String>,
    /// Original source `registreringsdatoEnhetsregisteret` value.
    #[serde(rename = "registreringsdatoEnhetsregisteret")]
    pub registreringsdato_enhetsregisteret: Option<String>,
    /// Original source `registreringsdatoForetaksregisteret` value.
    #[serde(rename = "registreringsdatoForetaksregisteret")]
    pub registreringsdato_foretaksregisteret: Option<String>,
    /// Original source `maalform` value.
    #[serde(rename = "maalform")]
    pub maalform: Option<String>,
    /// Original source `hjemmeside` value.
    #[serde(rename = "hjemmeside")]
    pub hjemmeside: Option<String>,
    /// Original source `naeringskode1` value.
    #[serde(rename = "naeringskode1")]
    pub naeringskode1: Option<NoClassification>,
    /// Original source `naeringskode2` value.
    #[serde(rename = "naeringskode2")]
    pub naeringskode2: Option<NoClassification>,
    /// Original source `naeringskode3` value.
    #[serde(rename = "naeringskode3")]
    pub naeringskode3: Option<NoClassification>,
}
/// Exact matched Norwegian entity observation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct NoCompanyLookupResult {
    /// Complete organisation number.
    pub tax_id: super::NoBusinessId,
    /// Original entity name.
    pub company_name: String,
    /// Typed native source fields.
    pub details: NoCompanyDetails,
}
