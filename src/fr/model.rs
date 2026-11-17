/// Native headquarters fields; administrative and country codes remain uninterpreted.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct FrHeadOffice {
    /// Original source `siret` value.
    #[serde(alias = "siret")]
    pub siret: Option<String>,
    /// Original source `adresse` value.
    #[serde(alias = "adresse")]
    pub adresse: Option<String>,
    /// Original source `numero_voie` value.
    #[serde(alias = "numero_voie")]
    pub numero_voie: Option<String>,
    /// Original source `indice_repetition` value.
    #[serde(alias = "indice_repetition")]
    pub indice_repetition: Option<String>,
    /// Original source `type_voie` value.
    #[serde(alias = "type_voie")]
    pub type_voie: Option<String>,
    /// Original source `libelle_voie` value.
    #[serde(alias = "libelle_voie")]
    pub libelle_voie: Option<String>,
    /// Original source `complement_adresse` value.
    #[serde(alias = "complement_adresse")]
    pub complement_adresse: Option<String>,
    /// Original source `distribution_speciale` value.
    #[serde(alias = "distribution_speciale")]
    pub distribution_speciale: Option<String>,
    /// Original source `code_postal` value.
    #[serde(alias = "code_postal")]
    pub code_postal: Option<String>,
    /// Original source `libelle_commune` value.
    #[serde(alias = "libelle_commune")]
    pub libelle_commune: Option<String>,
    /// Original source `commune` value.
    #[serde(alias = "commune")]
    pub commune: Option<String>,
    /// Original source `cedex` value.
    #[serde(alias = "cedex")]
    pub cedex: Option<String>,
    /// Original source `libelle_cedex` value.
    #[serde(alias = "libelle_cedex")]
    pub libelle_cedex: Option<String>,
    /// Original source `code_pays_etranger` value.
    #[serde(alias = "code_pays_etranger")]
    pub code_pays_etranger: Option<String>,
    /// Original source `libelle_pays_etranger` value.
    #[serde(alias = "libelle_pays_etranger")]
    pub libelle_pays_etranger: Option<String>,
    /// Original source `libelle_commune_etranger` value.
    #[serde(alias = "libelle_commune_etranger")]
    pub libelle_commune_etranger: Option<String>,
    /// Original source `departement` value.
    #[serde(alias = "departement")]
    pub departement: Option<String>,
    /// Original source `region` value.
    #[serde(alias = "region")]
    pub region: Option<String>,
    /// Original source `etat_administratif` value.
    #[serde(alias = "etat_administratif")]
    pub etat_administratif: Option<String>,
    /// Original source `date_creation` value.
    #[serde(alias = "date_creation")]
    pub date_creation: Option<String>,
    /// Original source `date_fermeture` value.
    #[serde(alias = "date_fermeture")]
    pub date_fermeture: Option<String>,
    /// Original source `statut_diffusion_etablissement` value.
    #[serde(alias = "statut_diffusion_etablissement")]
    pub statut_diffusion_etablissement: Option<String>,
    /// Original source `est_siege` value.
    #[serde(alias = "est_siege")]
    pub est_siege: Option<bool>,
}
/// Selected native public-company fields from DINUM, preserving source spelling.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct FrCompanyDetails {
    /// Original source `siren` value.
    #[serde(alias = "siren")]
    pub siren: String,
    /// Original source `nom_complet` value.
    #[serde(alias = "nom_complet")]
    pub nom_complet: String,
    /// Original source `siren_conserve` value.
    #[serde(alias = "siren_conserve")]
    pub siren_conserve: Option<String>,
    /// Original source `nom_raison_sociale` value.
    #[serde(alias = "nom_raison_sociale")]
    pub nom_raison_sociale: Option<String>,
    /// Original source `sigle` value.
    #[serde(alias = "sigle")]
    pub sigle: Option<String>,
    /// Original source `activite_principale` value.
    #[serde(alias = "activite_principale")]
    pub activite_principale: Option<String>,
    /// Original source `categorie_entreprise` value.
    #[serde(alias = "categorie_entreprise")]
    pub categorie_entreprise: Option<String>,
    /// Original source `date_creation` value.
    #[serde(alias = "date_creation")]
    pub date_creation: Option<String>,
    /// Original source `date_fermeture` value.
    #[serde(alias = "date_fermeture")]
    pub date_fermeture: Option<String>,
    /// Original source `date_mise_a_jour` value.
    #[serde(alias = "date_mise_a_jour")]
    pub date_mise_a_jour: Option<String>,
    /// Original source `date_mise_a_jour_insee` value.
    #[serde(alias = "date_mise_a_jour_insee")]
    pub date_mise_a_jour_insee: Option<String>,
    /// Original source `date_mise_a_jour_rne` value.
    #[serde(alias = "date_mise_a_jour_rne")]
    pub date_mise_a_jour_rne: Option<String>,
    /// Original source `etat_administratif` value.
    #[serde(alias = "etat_administratif")]
    pub etat_administratif: Option<String>,
    /// Original source `nature_juridique` value.
    #[serde(alias = "nature_juridique")]
    pub nature_juridique: Option<String>,
    /// Original source `statut_diffusion` value.
    #[serde(alias = "statut_diffusion")]
    pub statut_diffusion: Option<String>,
    /// Original source `siege` value.
    #[serde(alias = "siege")]
    pub siege: Option<FrHeadOffice>,
}
/// Exact matched SIREN observation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct FrCompanyLookupResult {
    /// Complete nine-digit company identifier.
    pub tax_id: super::FrBusinessId,
    /// Source display name, without generated translation.
    pub company_name: String,
    /// Typed native source fields.
    pub details: FrCompanyDetails,
}
