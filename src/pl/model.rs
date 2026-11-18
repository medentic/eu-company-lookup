/// Selected current or full P/S-register extract header.
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
    /// Unrecognized section-six field names; their presence prevents an active inference.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unmapped_lifecycle_fields: Vec<String>,
    /// Structured direction and source parties for each retained reorganization.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub corporate_events: Vec<PlCorporateEvent>,
    /// Terminal removal evidence from a full extract, never inferred from a merger mention.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_extract_removal: Option<PlRemoval>,
    /// Selected lifecycle facts from the current extract's sixth section.
    pub dzial6: Option<PlLifecycleSection>,
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
/// Exact matched Polish KRS P/S observation or supported removal recovery.
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

/// Selected sixth-section lifecycle facts; officer and ownership data are excluded.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PlLifecycleSection {
    /// Dissolution circumstance and description; not proof of removal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rozwiazanie_uniewaznienie: Option<PlDissolution>,
    /// Native merger, division and transformation entries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polaczenie_podzial_przeksztalcenie: Option<Vec<PlReorganization>>,
    /// Suspension and resumption dates from the current extract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zawieszenie_wznowienie: Option<Vec<PlSuspension>>,
    /// Liquidation opening and ending entries.
    #[serde(rename = "likwidacja")]
    pub likwidacja: Option<Vec<PlLiquidation>>,
    /// Bankruptcy proceeding entries.
    #[serde(rename = "postepowanieUpadlosciowe")]
    pub postepowanie_upadlosciowe: Option<Vec<PlBankruptcy>>,
    /// Restructuring proceeding entries.
    #[serde(
        rename = "postepowanieRestrukturyzacyjneNaprawczePrzymusowaRestrukturyzacjaUporzadkowanaLikwidacja"
    )]
    pub postepowanie_restrukturyzacyjne_naprawcze_przymusowa_restrukturyzacja_uporzadkowana_likwidacja:
        Option<Vec<PlRestructuring>>,
}

/// Native liquidation entry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PlLiquidation {
    /// Number of source liquidator entries, without officer identities.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub liquidator_count: Option<usize>,
    /// Opening information.
    #[serde(rename = "otwarcieLikwidacji")]
    pub otwarcie_likwidacji: Option<String>,
    /// Ending information; ending does not itself prove company deletion.
    #[serde(rename = "zakonczenieLikwidacji")]
    pub zakonczenie_likwidacji: Option<String>,
}

/// Native bankruptcy entry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PlBankruptcy {
    /// Opening order.
    #[serde(rename = "informacjaOOgloszeniuUpadlosci")]
    pub informacja_o_ogloszeniu_upadlosci: Option<PlProceedingOrder>,
    /// Ending information.
    #[serde(rename = "opisZakonczeniaProcesuUpadlosci")]
    pub opis_zakonczenia_procesu_upadlosci: Option<PlProceedingEnd>,
}

/// Native restructuring entry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PlRestructuring {
    /// Opening order.
    #[serde(
        rename = "otwarciePostepowaniaRestrukturyzacyjnegoNaprawczegoPrzymusowejRestrukturyzacjiUporzadkowanejLikwidacji"
    )]
    pub otwarcie_postepowania_restrukturyzacyjnego_naprawczego_przymusowej_restrukturyzacji_uporzadkowanej_likwidacji:
        Option<PlProceedingOrder>,
    /// Ending information.
    #[serde(
        rename = "zakonczeniePostepowaniaRestrukturyzacyjnegoNaprawczegoPrzymusowejRestrukturyzacjiUporzadkowanejLikwidacji"
    )]
    pub zakonczenie_postepowania_restrukturyzacyjnego_naprawczego_przymusowej_restrukturyzacji_uporzadkowanej_likwidacji:
        Option<PlProceedingEnd>,
}

/// Selected native court-order facts.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PlProceedingOrder {
    /// Original order date.
    #[serde(rename = "data")]
    pub data: Option<String>,
    /// Original enforceability date.
    #[serde(rename = "dataNadaniaKlauzuliWykonalnosci")]
    pub data_nadania_klauzuli_wykonalnosci: Option<String>,
    /// Original proceeding type.
    #[serde(rename = "rodzajPostepowania")]
    pub rodzaj_postepowania: Option<String>,
}

/// Selected native ending facts; unknown ending fields prevent an ongoing inference.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PlProceedingEnd {
    /// Original ending description.
    #[serde(rename = "opisSposobuZakonczeniaPostepowania")]
    pub opis_sposobu_zakonczenia_postepowania: Option<String>,
    /// Original ending date.
    #[serde(rename = "dataZakonczeniaPostepowania")]
    pub data_zakonczenia_postepowania: Option<String>,
    /// Unrecognized ending fields, retained to avoid inferring an ongoing proceeding.
    #[serde(flatten)]
    pub additional: std::collections::BTreeMap<String, serde_json::Value>,
}

/// Native suspension interval; an absent resumption does not mean deletion.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PlSuspension {
    /// Original suspension date.
    pub data_zawieszenia: Option<String>,
    /// Original resumption date.
    pub data_wznowienia: Option<String>,
}

/// Native reorganization entry. Descriptions are preserved, not interpreted as completion.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PlReorganization {
    /// S-register description spelling, retained without interpreting names as identifiers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opis_podzialu_polaczenia_przeksztalcenia: Option<String>,
    /// Exact direction/type label.
    pub okreslenie_okolicznosci: Option<String>,
    /// Original description, potentially including proposals or historical transactions.
    pub opis_polaczenia_podzialu_przeksztalcenia: Option<String>,
    /// Entities formed or receiving assets; not always merger successors.
    pub podmioty_powstale_przejmujace: Option<Vec<PlMergerParty>>,
    /// Entities whose assets are acquired by this company.
    pub podmioty_przejmowane: Option<Vec<PlMergerParty>>,
}
/// Selected corporate party fields; no officer or personal data.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PlMergerParty {
    /// Native party identifiers; REGON is not promoted to KRS.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identyfikator: Option<PlSourceIdentifiers>,
    /// Original company name and form.
    pub nazwa: Option<String>,
    /// Original country/register description.
    pub kraj_nazwa_rejestru: Option<String>,
    /// Original full-extract register number, including non-KRS registers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub numer_w_rejestrze_albo_ewidencji: Option<String>,
    /// Native KRS field; placeholders remain native and are excluded from successor IDs.
    pub krs: Option<PlMergerKrs>,
}
/// Native nested KRS identifier.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct PlMergerKrs {
    /// Original identifier, including any provider placeholder.
    pub krs: Option<String>,
}
/// Company's role in a particular transaction, not its present lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum PlReorganizationRole {
    /// This company acquires another company.
    Acquirer,
    /// Another company acquires this company; completion requires separate evidence.
    Absorbed,
    /// The structured resulting-party identifier is this company.
    ResultingEntity,
    /// This company acquires only part of another company's assets.
    AssetRecipient,
    /// Part of this company's assets is separated; this does not prove closure.
    PartialDivision,
    /// Change of legal form, not automatically a merger or closure.
    Transformation,
    /// Unsupported or conflicting role evidence.
    Unresolved,
}
/// Interpreted role plus original structured reorganization fields.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PlCorporateEvent {
    /// Direction in this event only; not proof of current active/closed status.
    pub role: PlReorganizationRole,
    /// Exact non-placeholder KRS numbers of named receivers; the event may still be proposed.
    pub successor_krs: Vec<String>,
    /// Original selected entry fields.
    pub source: PlReorganization,
}
/// Terminal register removal from an identity-matched full extract; finality is explicit.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PlRemoval {
    /// Original removal entry number.
    pub entry_number: u64,
    /// Original removal description.
    pub description: String,
    /// Original removal date (DD.MM.YYYY).
    pub entry_date: String,
    /// Original finality date (DD.MM.YYYY).
    pub final_date: Option<String>,
    /// Original non-finality marker; a pending/contradictory removal is not a confirmed closure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_final: Option<String>,
}

/// Selected dissolution facts, retained separately from register removal.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PlDissolution {
    /// Original circumstance label.
    pub okreslenie_okolicznosci: Option<String>,
    /// Original resolution or court-order description.
    pub informacja_o_rozwiazaniu_uniewaznieniu: Option<String>,
    /// Unrecognized fields prevent interpreting this as historical dissolution.
    #[serde(flatten)]
    pub additional: std::collections::BTreeMap<String, serde_json::Value>,
}
