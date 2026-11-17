use crate::CzBusinessId;
use serde::{Deserialize, Serialize};
/// Matched ARES economic-subject observation; not a certified registry extract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CzCompanyLookupResult {
    /// Full matched IČO.
    pub tax_id: CzBusinessId,
    /// Source company name.
    pub company_name: String,
    /// Registered-office source components.
    pub registered_office: Option<CzRegistryAddress>,
    /// Original ARES `pravniForma` value; no inferred semantics.
    pub legal_form_code: Option<String>,
    /// Original ARES `pravniFormaRos` value; no inferred semantics.
    pub ros_legal_form_code: Option<String>,
    /// Original ARES `dic` value; no inferred semantics.
    pub tax_identifier: Option<String>,
    /// Original ARES `dicSkDph` value; no inferred semantics.
    pub vat_group_identifier: Option<String>,
    /// Original ARES `datumVzniku` value; no inferred semantics.
    pub created_date: Option<String>,
    /// Original ARES `datumZaniku` value; no inferred semantics.
    pub ceased_date: Option<String>,
    /// Original ARES `datumAktualizace` value; no inferred semantics.
    pub updated_date: Option<String>,
    /// Original ARES `primarniZdroj` value; no inferred semantics.
    pub primary_source: Option<String>,
    /// Status in each source register, not company lifecycle.
    pub source_registrations: Option<CzSourceRegistrations>,
}
/// Typed ARES Adresa source fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CzRegistryAddress {
    /// Original source field `kodStatu`.
    #[serde(rename = "kodStatu")]
    pub kod_statu: Option<String>,
    /// Original source field `nazevStatu`.
    #[serde(rename = "nazevStatu")]
    pub nazev_statu: Option<String>,
    /// Original source field `kodKraje`.
    #[serde(rename = "kodKraje")]
    pub kod_kraje: Option<i64>,
    /// Original source field `nazevKraje`.
    #[serde(rename = "nazevKraje")]
    pub nazev_kraje: Option<String>,
    /// Original source field `kodOkresu`.
    #[serde(rename = "kodOkresu")]
    pub kod_okresu: Option<i64>,
    /// Original source field `nazevOkresu`.
    #[serde(rename = "nazevOkresu")]
    pub nazev_okresu: Option<String>,
    /// Original source field `kodObce`.
    #[serde(rename = "kodObce")]
    pub kod_obce: Option<i64>,
    /// Original source field `nazevObce`.
    #[serde(rename = "nazevObce")]
    pub nazev_obce: Option<String>,
    /// Original source field `kodSpravnihoObvodu`.
    #[serde(rename = "kodSpravnihoObvodu")]
    pub kod_spravniho_obvodu: Option<i64>,
    /// Original source field `nazevSpravnihoObvodu`.
    #[serde(rename = "nazevSpravnihoObvodu")]
    pub nazev_spravniho_obvodu: Option<String>,
    /// Original source field `kodMestskehoObvodu`.
    #[serde(rename = "kodMestskehoObvodu")]
    pub kod_mestskeho_obvodu: Option<i64>,
    /// Original source field `nazevMestskehoObvodu`.
    #[serde(rename = "nazevMestskehoObvodu")]
    pub nazev_mestskeho_obvodu: Option<String>,
    /// Original source field `kodMestskeCastiObvodu`.
    #[serde(rename = "kodMestskeCastiObvodu")]
    pub kod_mestske_casti_obvodu: Option<i64>,
    /// Original source field `kodUlice`.
    #[serde(rename = "kodUlice")]
    pub kod_ulice: Option<i64>,
    /// Original source field `nazevMestskeCastiObvodu`.
    #[serde(rename = "nazevMestskeCastiObvodu")]
    pub nazev_mestske_casti_obvodu: Option<String>,
    /// Original source field `nazevUlice`.
    #[serde(rename = "nazevUlice")]
    pub nazev_ulice: Option<String>,
    /// Original source field `cisloDomovni`.
    #[serde(rename = "cisloDomovni")]
    pub cislo_domovni: Option<i64>,
    /// Original source field `doplnekAdresy`.
    #[serde(rename = "doplnekAdresy")]
    pub doplnek_adresy: Option<String>,
    /// Original source field `kodCastiObce`.
    #[serde(rename = "kodCastiObce")]
    pub kod_casti_obce: Option<i64>,
    /// Original source field `cisloOrientacni`.
    #[serde(rename = "cisloOrientacni")]
    pub cislo_orientacni: Option<i64>,
    /// Original source field `cisloOrientacniPismeno`.
    #[serde(rename = "cisloOrientacniPismeno")]
    pub cislo_orientacni_pismeno: Option<String>,
    /// Original source field `nazevCastiObce`.
    #[serde(rename = "nazevCastiObce")]
    pub nazev_casti_obce: Option<String>,
    /// Original source field `kodAdresnihoMista`.
    #[serde(rename = "kodAdresnihoMista")]
    pub kod_adresniho_mista: Option<i64>,
    /// Original source field `psc`.
    #[serde(rename = "psc")]
    pub psc: Option<i64>,
    /// Original source field `textovaAdresa`.
    #[serde(rename = "textovaAdresa")]
    pub textova_adresa: Option<String>,
    /// Original source field `cisloDoAdresy`.
    #[serde(rename = "cisloDoAdresy")]
    pub cislo_do_adresy: Option<String>,
    /// Original source field `standardizaceAdresy`.
    #[serde(rename = "standardizaceAdresy")]
    pub standardizace_adresy: Option<bool>,
    /// Original source field `pscTxt`.
    #[serde(rename = "pscTxt")]
    pub psc_txt: Option<String>,
    /// Original source field `typCisloDomovni`.
    #[serde(rename = "typCisloDomovni")]
    pub typ_cislo_domovni: Option<i64>,
}
/// Typed ARES SeznamRegistraci source fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CzSourceRegistrations {
    /// Original source field `stavZdrojeRos`.
    #[serde(rename = "stavZdrojeRos")]
    pub stav_zdroje_ros: Option<String>,
    /// Original source field `stavZdrojeVr`.
    #[serde(rename = "stavZdrojeVr")]
    pub stav_zdroje_vr: Option<String>,
    /// Original source field `stavZdrojeRes`.
    #[serde(rename = "stavZdrojeRes")]
    pub stav_zdroje_res: Option<String>,
    /// Original source field `stavZdrojeRzp`.
    #[serde(rename = "stavZdrojeRzp")]
    pub stav_zdroje_rzp: Option<String>,
    /// Original source field `stavZdrojeNrpzs`.
    #[serde(rename = "stavZdrojeNrpzs")]
    pub stav_zdroje_nrpzs: Option<String>,
    /// Original source field `stavZdrojeRpsh`.
    #[serde(rename = "stavZdrojeRpsh")]
    pub stav_zdroje_rpsh: Option<String>,
    /// Original source field `stavZdrojeRcns`.
    #[serde(rename = "stavZdrojeRcns")]
    pub stav_zdroje_rcns: Option<String>,
    /// Original source field `stavZdrojeSzr`.
    #[serde(rename = "stavZdrojeSzr")]
    pub stav_zdroje_szr: Option<String>,
    /// Original source field `stavZdrojeDph`.
    #[serde(rename = "stavZdrojeDph")]
    pub stav_zdroje_dph: Option<String>,
    /// Original source field `stavZdrojeSkDph`.
    #[serde(rename = "stavZdrojeSkDph")]
    pub stav_zdroje_sk_dph: Option<String>,
    /// Original source field `stavZdrojeSd`.
    #[serde(rename = "stavZdrojeSd")]
    pub stav_zdroje_sd: Option<String>,
    /// Original source field `stavZdrojeIr`.
    #[serde(rename = "stavZdrojeIr")]
    pub stav_zdroje_ir: Option<String>,
    /// Original source field `stavZdrojeCeu`.
    #[serde(rename = "stavZdrojeCeu")]
    pub stav_zdroje_ceu: Option<String>,
    /// Original source field `stavZdrojeRs`.
    #[serde(rename = "stavZdrojeRs")]
    pub stav_zdroje_rs: Option<String>,
    /// Original source field `stavZdrojeRed`.
    #[serde(rename = "stavZdrojeRed")]
    pub stav_zdroje_red: Option<String>,
    /// Original source field `stavZdrojeMonitor`.
    #[serde(rename = "stavZdrojeMonitor")]
    pub stav_zdroje_monitor: Option<String>,
}

/// Native ARES codes and observations retained alongside the common company view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CzAresDetails {
    /// Native registered-office components.
    pub registered_office: Option<CzRegistryAddress>,
    /// Source legal form code.
    pub legal_form_code: Option<String>,
    /// ROS legal form code.
    pub ros_legal_form_code: Option<String>,
    /// Tax identifier, not by itself proof of VAT registration.
    pub tax_identifier: Option<String>,
    /// Separate VAT group identifier.
    pub vat_group_identifier: Option<String>,
    /// Source date of subject creation.
    pub created_date: Option<String>,
    /// Source date of subject cessation.
    pub ceased_date: Option<String>,
    /// Source record update date.
    pub updated_date: Option<String>,
    /// Primary source code.
    pub primary_source: Option<String>,
    /// Per-source registration states, not normalized lifecycle.
    pub source_registrations: Option<CzSourceRegistrations>,
}
/// Matched ARES economic-subject observation; not a certified registry extract.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]

pub(super) struct Wire {
    /// Full matched IČO.
    #[serde(rename = "ico")]
    pub tax_id: CzBusinessId,
    /// Source company name.
    #[serde(rename = "obchodniJmeno")]
    pub company_name: String,
    /// Registered-office source components.
    #[serde(rename = "sidlo")]
    pub registered_office: Option<CzRegistryAddress>,
    /// Original ARES `pravniForma` value; no inferred semantics.
    #[serde(rename = "pravniForma")]
    pub legal_form_code: Option<String>,
    /// Original ARES `pravniFormaRos` value; no inferred semantics.
    #[serde(rename = "pravniFormaRos")]
    pub ros_legal_form_code: Option<String>,
    /// Original ARES `dic` value; no inferred semantics.
    #[serde(rename = "dic")]
    pub tax_identifier: Option<String>,
    /// Original ARES `dicSkDph` value; no inferred semantics.
    #[serde(rename = "dicSkDph")]
    pub vat_group_identifier: Option<String>,
    /// Original ARES `datumVzniku` value; no inferred semantics.
    #[serde(rename = "datumVzniku")]
    pub created_date: Option<String>,
    /// Original ARES `datumZaniku` value; no inferred semantics.
    #[serde(rename = "datumZaniku")]
    pub ceased_date: Option<String>,
    /// Original ARES `datumAktualizace` value; no inferred semantics.
    #[serde(rename = "datumAktualizace")]
    pub updated_date: Option<String>,
    /// Original ARES `primarniZdroj` value; no inferred semantics.
    #[serde(rename = "primarniZdroj")]
    pub primary_source: Option<String>,
    /// Status in each source register, not company lifecycle.
    #[serde(rename = "seznamRegistraci")]
    pub source_registrations: Option<CzSourceRegistrations>,
}
impl From<Wire> for CzCompanyLookupResult {
    fn from(v: Wire) -> Self {
        Self {
            tax_id: v.tax_id,
            company_name: v.company_name,
            registered_office: v.registered_office,
            legal_form_code: v.legal_form_code,
            ros_legal_form_code: v.ros_legal_form_code,
            tax_identifier: v.tax_identifier,
            vat_group_identifier: v.vat_group_identifier,
            created_date: v.created_date,
            ceased_date: v.ceased_date,
            updated_date: v.updated_date,
            primary_source: v.primary_source,
            source_registrations: v.source_registrations,
        }
    }
}
