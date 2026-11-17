/// Current business-register extract header.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PlExtractHeader {
    /// Original source `rejestr` value.
    #[serde(rename = "rejestr")]
    pub rejestr: String,
    /// Original source `numerKRS` value.
    #[serde(rename = "numerKRS")]
    pub numer_krs: String,
    /// Original source `dataCzasOdpisu` value.
    #[serde(rename = "dataCzasOdpisu")]
    pub data_czas_odpisu: Option<String>,
    /// Original source `stanZDnia` value.
    #[serde(rename = "stanZDnia")]
    pub stan_z_dnia: Option<String>,
    /// Original source `dataRejestracjiWKRS` value.
    #[serde(rename = "dataRejestracjiWKRS")]
    pub data_rejestracji_w_krs: Option<String>,
    /// Original source `dataOstatniegoWpisu` value.
    #[serde(rename = "dataOstatniegoWpisu")]
    pub data_ostatniego_wpisu: Option<String>,
    /// Original source `stanPozycji` value.
    #[serde(rename = "stanPozycji")]
    pub stan_pozycji: Option<u64>,
}
/// Native fiscal/statistical identifiers, not VIES evidence.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PlSourceIdentifiers {
    /// Original source `nip` value.
    #[serde(rename = "nip")]
    pub nip: Option<String>,
    /// Original source `regon` value.
    #[serde(rename = "regon")]
    pub regon: Option<String>,
}
/// Selected company identity fields.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PlSubject {
    /// Foreign undertaking name and form, when this is a Polish branch.
    #[serde(rename = "nazwaFormaPrawnaPrzedsiebiorcyZagranicznego")]
    pub foreign_undertaking_name: Option<String>,
    /// Original foreign register description, not a Polish identity override.
    #[serde(rename = "innyRejestr")]
    pub foreign_register: Option<String>,
    /// Original source `nazwa` value.
    #[serde(rename = "nazwa")]
    pub nazwa: String,
    /// Original source `formaPrawna` value.
    #[serde(rename = "formaPrawna")]
    pub forma_prawna: Option<String>,
    /// Original source `identyfikatory` value.
    #[serde(rename = "identyfikatory")]
    pub identyfikatory: Option<PlSourceIdentifiers>,
}
/// Native legal seat, retained separately from address.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PlSeat {
    /// Original source `kraj` value.
    #[serde(rename = "kraj")]
    pub kraj: Option<String>,
    /// Original source `wojewodztwo` value.
    #[serde(rename = "wojewodztwo")]
    pub wojewodztwo: Option<String>,
    /// Original source `powiat` value.
    #[serde(rename = "powiat")]
    pub powiat: Option<String>,
    /// Original source `gmina` value.
    #[serde(rename = "gmina")]
    pub gmina: Option<String>,
    /// Original source `miejscowosc` value.
    #[serde(rename = "miejscowosc")]
    pub miejscowosc: Option<String>,
}
/// Original registered-office components; no invented full text.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PlAddress {
    /// Original province text, when supplied in a foreign head-office address.
    pub wojewodztwo: Option<String>,
    /// Original source `ulica` value.
    #[serde(rename = "ulica")]
    pub ulica: Option<String>,
    /// Original source `nrDomu` value.
    #[serde(rename = "nrDomu")]
    pub nr_domu: Option<String>,
    /// Original source `nrLokalu` value.
    #[serde(rename = "nrLokalu")]
    pub nr_lokalu: Option<String>,
    /// Original source `miejscowosc` value.
    #[serde(rename = "miejscowosc")]
    pub miejscowosc: Option<String>,
    /// Original source `kodPocztowy` value.
    #[serde(rename = "kodPocztowy")]
    pub kod_pocztowy: Option<String>,
    /// Original source `poczta` value.
    #[serde(rename = "poczta")]
    pub poczta: Option<String>,
    /// Original source `kraj` value.
    #[serde(rename = "kraj")]
    pub kraj: Option<String>,
}
/// Separate native legal seat and address.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PlRegisteredOffice {
    /// Foreign undertaking head office; never substituted for the Polish branch.
    #[serde(rename = "siedzibaIAdresZakladuGlownego")]
    pub foreign_head_office: Option<PlAddress>,
    /// Original source `siedziba` value.
    #[serde(rename = "siedziba")]
    pub siedziba: Option<PlSeat>,
    /// Original source `adres` value.
    #[serde(rename = "adres")]
    pub adres: Option<PlAddress>,
}
/// Selected source extract fields.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PlCompanyDetails {
    /// True only when the source uses its foreign-branch subject section.
    pub foreign_branch: bool,
    /// Original source `naglowek` value.
    #[serde(rename = "naglowek")]
    pub naglowek: PlExtractHeader,
    /// Original source `danePodmiotu` value.
    #[serde(rename = "danePodmiotu")]
    pub dane_podmiotu: PlSubject,
    /// Original source `siedzibaIAdres` value.
    #[serde(rename = "siedzibaIAdres")]
    pub siedziba_i_adres: Option<PlRegisteredOffice>,
}
/// Exact matched current Polish KRS business observation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PlCompanyLookupResult {
    /// Full requested KRS number, distinct from NIP.
    pub tax_id: super::PlBusinessId,
    /// Original company name.
    pub company_name: String,
    /// Selected native extract fields.
    pub details: PlCompanyDetails,
}
