use crate::{
    BgRegistryClient, BgRegistryConfig, ClientConfig, CompanyLookupResult, IdentifierScheme,
    JurisdictionCapabilities, LookupError, Operation, Provider, RoRegistryClient, RoRegistryConfig,
    VatLookupResult,
};

/// Provider-independent lookup facade. Clones share each enabled adapter's
/// admission state. No automatic retry or cross-provider fallback is introduced.
#[derive(Clone, Debug)]
pub struct Client {
    vies: Option<crate::vies::ViesClient>,
    bg: Option<crate::bg::BgCompanyClient>,
    ro: Option<RoRegistryClient>,
    cz: Option<crate::CzRegistryClient>,
    at: Option<crate::AtRegistryClient>,
    be: Option<crate::BeRegistryClient>,
    fi: Option<crate::FiRegistryClient>,
    fr: Option<crate::FrRegistryClient>,
    is: Option<crate::IsRegistryClient>,
    ie: Option<crate::IeRegistryClient>,
    lt: Option<crate::LtRegistryClient>,
    lv: Option<crate::LvRegistryClient>,
    sk: Option<crate::SkRegistryClient>,
    pl: Option<crate::PlRegistryClient>,
    no: Option<crate::NoRegistryClient>,
    ee: Option<crate::EeRegistryClient>,
    gr: Option<crate::GrRegistryClient>,
}

/// Named adapter configuration. Defaults enable all implemented lookup adapters;
/// use `empty()` to explicitly select only the adapters your application needs.
#[derive(Clone, Debug)]
pub struct ClientBuilder {
    vies: Option<ClientConfig>,
    bg: Option<BgRegistryConfig>,
    ro: Option<RoRegistryConfig>,
    cz: Option<crate::CzRegistryConfig>,
    at: Option<crate::AtRegistryConfig>,
    be: Option<crate::BeRegistryConfig>,
    fi: Option<crate::FiRegistryConfig>,
    fr: Option<crate::FrRegistryConfig>,
    is: Option<crate::IsRegistryConfig>,
    ie: Option<crate::IeRegistryConfig>,
    lt: Option<crate::LtRegistryConfig>,
    lv: Option<crate::LvRegistryConfig>,
    sk: Option<crate::SkRegistryConfig>,
    pl: Option<crate::PlRegistryConfig>,
    no: Option<crate::NoRegistryConfig>,
    ee: Option<crate::EeRegistryConfig>,
    gr: Option<crate::GrRegistryConfig>,
}
impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            vies: Some(Default::default()),
            bg: Some(Default::default()),
            ro: Some(Default::default()),
            cz: Some(Default::default()),
            at: Some(Default::default()),
            be: Some(Default::default()),
            fi: Some(Default::default()),
            fr: Some(Default::default()),
            is: Some(Default::default()),
            ie: Some(Default::default()),
            lt: Some(Default::default()),
            lv: Some(Default::default()),
            sk: Some(Default::default()),
            pl: Some(Default::default()),
            no: Some(Default::default()),
            ee: Some(Default::default()),
            gr: Some(Default::default()),
        }
    }
}

impl ClientBuilder {
    /// Start with no adapters enabled. Building this configuration creates no HTTP clients.
    pub fn empty() -> Self {
        Self {
            vies: None,
            bg: None,
            ro: None,
            cz: None,
            at: None,
            be: None,
            fi: None,
            fr: None,
            is: None,
            ie: None,
            lt: None,
            lv: None,
            sk: None,
            pl: None,
            no: None,
            ee: None,
            gr: None,
        }
    }
    /// Enable standalone VIES with these transport/admission bounds.
    /// BG company verification shares this client and its admission limits.
    pub fn vies(mut self, config: ClientConfig) -> Self {
        self.vies = Some(config);
        self
    }
    /// Enable Bulgarian register lookup, retaining Commercial Register/BULSTAT fallback.
    pub fn bulgarian_registry(mut self, config: BgRegistryConfig) -> Self {
        self.bg = Some(config);
        self
    }
    /// Enable Romanian ANAF lookup with independent bounds.
    pub fn romanian_registry(mut self, config: RoRegistryConfig) -> Self {
        self.ro = Some(config);
        self
    }
    /// Enable anonymous Greek GEMI company lookup.
    pub fn greek_registry(mut self, config: crate::GrRegistryConfig) -> Self {
        self.gr = Some(config);
        self
    }
    /// Disable Greek GEMI company lookup.
    pub fn without_greek_registry(mut self) -> Self {
        self.gr = None;
        self
    }
    /// Enable anonymous Belgian NBB Consult lookup.
    pub fn belgian_registry(mut self, config: crate::BeRegistryConfig) -> Self {
        self.be = Some(config);
        self
    }
    /// Disable Belgian NBB Consult lookup.
    pub fn without_belgian_registry(mut self) -> Self {
        self.be = None;
        self
    }
    /// Enable anonymous Finnish PRH YTJ lookup.
    pub fn finnish_registry(mut self, config: crate::FiRegistryConfig) -> Self {
        self.fi = Some(config);
        self
    }
    /// Disable Finnish PRH YTJ lookup.
    pub fn without_finnish_registry(mut self) -> Self {
        self.fi = None;
        self
    }
    /// Enable anonymous French API Recherche Entreprises lookup.
    pub fn french_registry(mut self, config: crate::FrRegistryConfig) -> Self {
        self.fr = Some(config);
        self
    }
    /// Disable French API Recherche Entreprises lookup.
    pub fn without_french_registry(mut self) -> Self {
        self.fr = None;
        self
    }
    /// Enable anonymous Icelandic public company-register lookup.
    pub fn icelandic_registry(mut self, config: crate::IsRegistryConfig) -> Self {
        self.is = Some(config);
        self
    }
    /// Disable Icelandic public company-register lookup.
    pub fn without_icelandic_registry(mut self) -> Self {
        self.is = None;
        self
    }
    /// Enable anonymous Irish CRO open-data lookup.
    pub fn irish_registry(mut self, config: crate::IeRegistryConfig) -> Self {
        self.ie = Some(config);
        self
    }
    /// Disable Irish CRO open-data lookup.
    pub fn without_irish_registry(mut self) -> Self {
        self.ie = None;
        self
    }
    /// Enable anonymous Lithuanian legal-entity open-data lookup.
    pub fn lithuanian_registry(mut self, config: crate::LtRegistryConfig) -> Self {
        self.lt = Some(config);
        self
    }
    /// Disable Lithuanian legal-entity open-data lookup.
    pub fn without_lithuanian_registry(mut self) -> Self {
        self.lt = None;
        self
    }
    /// Enable anonymous Latvian registry open-data lookup.
    pub fn latvian_registry(mut self, config: crate::LvRegistryConfig) -> Self {
        self.lv = Some(config);
        self
    }
    /// Disable Latvian registry open-data lookup.
    pub fn without_latvian_registry(mut self) -> Self {
        self.lv = None;
        self
    }
    /// Enable anonymous Slovak RÚZ accounting-entity lookup.
    pub fn slovak_registry(mut self, config: crate::SkRegistryConfig) -> Self {
        self.sk = Some(config);
        self
    }
    /// Disable Slovak RÚZ accounting-entity lookup.
    pub fn without_slovak_registry(mut self) -> Self {
        self.sk = None;
        self
    }
    /// Enable anonymous Polish KRS lookup.
    pub fn polish_registry(mut self, config: crate::PlRegistryConfig) -> Self {
        self.pl = Some(config);
        self
    }
    /// Disable Polish KRS lookup.
    pub fn without_polish_registry(mut self) -> Self {
        self.pl = None;
        self
    }
    /// Enable anonymous Norwegian Enhetsregisteret lookup.
    pub fn norwegian_registry(mut self, config: crate::NoRegistryConfig) -> Self {
        self.no = Some(config);
        self
    }
    /// Disable Norwegian Enhetsregisteret lookup.
    pub fn without_norwegian_registry(mut self) -> Self {
        self.no = None;
        self
    }
    /// Enable anonymous Estonian RIK public registry lookup.
    pub fn estonian_registry(mut self, config: crate::EeRegistryConfig) -> Self {
        self.ee = Some(config);
        self
    }
    /// Disable Estonian RIK public registry lookup.
    pub fn without_estonian_registry(mut self) -> Self {
        self.ee = None;
        self
    }
    /// Enable anonymous Austrian Firmenbuch lookup.
    pub fn austrian_registry(mut self, config: crate::AtRegistryConfig) -> Self {
        self.at = Some(config);
        self
    }
    /// Disable Austrian Firmenbuch lookup.
    pub fn without_austrian_registry(mut self) -> Self {
        self.at = None;
        self
    }
    /// Enable Czech ARES lookup.
    pub fn czech_registry(mut self, config: crate::CzRegistryConfig) -> Self {
        self.cz = Some(config);
        self
    }
    /// Disable Czech ARES lookup.
    pub fn without_czech_registry(mut self) -> Self {
        self.cz = None;
        self
    }
    /// Disable only standalone VAT lookup operations.
    /// BG company lookup still performs its required internal VIES check, using defaults.
    pub fn without_vies(mut self) -> Self {
        self.vies = None;
        self
    }
    /// Disable the Bulgarian register lookup, including its BULSTAT fallback.
    pub fn without_bulgarian_registry(mut self) -> Self {
        self.bg = None;
        self
    }
    /// Disable Romanian ANAF lookup.
    pub fn without_romanian_registry(mut self) -> Self {
        self.ro = None;
        self
    }
    /// Validate and construct only the enabled adapters. Makes no provider requests.
    pub fn build(self) -> Result<Client, LookupError> {
        let vies = self
            .vies
            .map(crate::vies::ViesClient::with_config)
            .transpose()?;
        let bg = self
            .bg
            .map(|config| -> Result<_, LookupError> {
                let verification = match &vies {
                    Some(client) => client.clone(),
                    None => crate::vies::ViesClient::with_config(Default::default())?,
                };
                Ok(crate::bg::BgCompanyClient::new(
                    BgRegistryClient::with_config(config)?,
                    verification,
                ))
            })
            .transpose()?;
        Ok(Client {
            is: self
                .is
                .map(crate::IsRegistryClient::with_config)
                .transpose()?,
            ie: self
                .ie
                .map(crate::IeRegistryClient::with_config)
                .transpose()?,
            lt: self
                .lt
                .map(crate::LtRegistryClient::with_config)
                .transpose()?,
            lv: self
                .lv
                .map(crate::LvRegistryClient::with_config)
                .transpose()?,
            sk: self
                .sk
                .map(crate::SkRegistryClient::with_config)
                .transpose()?,
            pl: self
                .pl
                .map(crate::PlRegistryClient::with_config)
                .transpose()?,
            no: self
                .no
                .map(crate::NoRegistryClient::with_config)
                .transpose()?,
            ee: self
                .ee
                .map(crate::EeRegistryClient::with_config)
                .transpose()?,
            fr: self
                .fr
                .map(crate::FrRegistryClient::with_config)
                .transpose()?,
            fi: self
                .fi
                .map(crate::FiRegistryClient::with_config)
                .transpose()?,
            gr: self
                .gr
                .map(crate::GrRegistryClient::with_config)
                .transpose()?,
            be: self
                .be
                .map(crate::BeRegistryClient::with_config)
                .transpose()?,
            at: self
                .at
                .map(crate::AtRegistryClient::with_config)
                .transpose()?,
            vies,
            bg,
            ro: self.ro.map(RoRegistryClient::with_config).transpose()?,
            cz: self
                .cz
                .map(crate::CzRegistryClient::with_config)
                .transpose()?,
        })
    }
}

fn disabled(operation: Operation, identifier_scheme: IdentifierScheme) -> LookupError {
    LookupError::from(crate::model::LookupFailure::OperationDisabled {
        operation,
        identifier_scheme,
    })
}

impl Client {
    /// ISO 3166-1 alpha-2 countries supported by company lookup.
    ///
    /// This lists compiled support; use `capabilities()` to inspect which
    /// providers are enabled on a particular client.
    pub const SUPPORTED_COMPANY_COUNTRIES: [&'static str; 16] = [
        "AT", "BE", "BG", "CZ", "EE", "GR", "FI", "FR", "IE", "LT", "LV", "PL", "RO", "SK", "IS",
        "NO",
    ];

    /// Configure adapters by name, starting with all current adapters enabled.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }
    /// Enable all implemented adapters with bounded defaults.
    pub fn new() -> Result<Self, LookupError> {
        Self::builder().build()
    }
    /// Customize VIES; registry adapters retain defaults.
    pub fn with_config(config: ClientConfig) -> Result<Self, LookupError> {
        Self::builder().vies(config).build()
    }
    /// Compatibility convenience for configuring VIES and Bulgaria.
    pub fn with_configs(config: ClientConfig, bg: BgRegistryConfig) -> Result<Self, LookupError> {
        Self::builder().vies(config).bulgarian_registry(bg).build()
    }
    /// Compatibility convenience; prefer named builder methods for new integrations.
    pub fn with_registry_configs(
        config: ClientConfig,
        bg: BgRegistryConfig,
        ro: RoRegistryConfig,
    ) -> Result<Self, LookupError> {
        Self::builder()
            .vies(config)
            .bulgarian_registry(bg)
            .romanian_registry(ro)
            .build()
    }
    /// Capabilities of adapters enabled on this client. Empty provider lists do
    /// not establish lookup support. This describes public operations, not internal
    /// dependencies: BG company lookup always uses VIES for eligible identifiers.
    /// Use the free `capabilities()` for compiled support.
    pub fn capabilities(&self) -> Vec<JurisdictionCapabilities> {
        let mut catalog = crate::capabilities();
        for country in &mut catalog {
            country.providers.retain(|p| match p.provider {
                Provider::Vies => self.vies.is_some(),
                Provider::BulgarianCommercialRegister | Provider::Bulstat => self.bg.is_some(),
                Provider::RomanianAnaf => self.ro.is_some(),
                Provider::CzechAres => self.cz.is_some(),
                Provider::AustrianFirmenbuch => self.at.is_some(),
                Provider::BelgianNbb => self.be.is_some(),
                Provider::FinnishPrh => self.fi.is_some(),
                Provider::FrenchRechercheEntreprises => self.fr.is_some(),
                Provider::IcelandicSkatturinn => self.is.is_some(),
                Provider::IrishCro => self.ie.is_some(),
                Provider::LithuanianJar => self.lt.is_some(),
                Provider::LatvianOpenData => self.lv.is_some(),
                Provider::SlovakRuz => self.sk.is_some(),
                Provider::PolishKrs => self.pl.is_some(),
                Provider::NorwegianBrreg => self.no.is_some(),
                Provider::EstonianRik => self.ee.is_some(),
                Provider::GreekGemi => self.gr.is_some(),
            });
        }
        catalog
    }
    /// VIES lookup; an explicitly disabled adapter returns a local typed error.
    pub async fn lookup_vat(&self, value: &str) -> Result<VatLookupResult, LookupError> {
        self.vies
            .as_ref()
            .ok_or_else(|| disabled(Operation::LookupVat, IdentifierScheme::Vat))?
            .lookup_vat(value)
            .await
    }
    /// Boolean convenience over the same VIES observation; failures never become false.
    pub async fn is_vat_valid(&self, value: &str) -> Result<bool, LookupError> {
        let vies = self
            .vies
            .as_ref()
            .ok_or_else(|| disabled(Operation::IsVatValid, IdentifierScheme::Vat))?;
        Ok(vies.lookup_vat(value).await?.valid)
    }
    /// Look up a national business identifier using its ISO 3166-1 alpha-2 country.
    ///
    /// Supports countries in [Self::SUPPORTED_COMPANY_COUNTRIES]
    /// (case-insensitive, surrounding whitespace ignored).
    /// Supply the national code without a VAT prefix. Leading zeroes are preserved.
    /// Returns a common company view with typed source details. Romanian lookups
    /// request today's fiscal status in Europe/Bucharest. BG nine-digit results are
    /// automatically checked by the BG adapter through VIES; errors propagate, adding its own
    /// deadline after the registry lookup. A valid, name-matched response fills VAT
    /// fields. Negative or unnamed/mismatched responses retain unknown VAT fields
    /// and remain available in `registryDetails.data.vatVerification`.
    /// Thirteen-digit branch IDs are not converted to VAT candidates.
    ///
    /// Unsupported countries return `UnsupportedIdentifierScheme`. For supported
    /// countries, disabled adapters return `OperationDisabled` before ID validation.
    pub async fn lookup_company(
        &self,
        value: &str,
        country: &str,
    ) -> Result<CompanyLookupResult, LookupError> {
        let country = country.trim().to_ascii_uppercase();
        if !Self::SUPPORTED_COMPANY_COUNTRIES.contains(&country.as_str()) {
            return Err(LookupError::from(
                crate::model::LookupFailure::UnsupportedIdentifierScheme,
            ));
        }
        match country.as_str() {
            "BG" => {
                company_adapter(&self.bg, IdentifierScheme::BgBusinessId)?
                    .lookup(value)
                    .await
            }
            "RO" => company_adapter(&self.ro, IdentifierScheme::RoBusinessId)?
                .lookup(value)
                .await
                .map(Into::into),
            "AT" => company_adapter(&self.at, IdentifierScheme::AtBusinessId)?
                .lookup(value)
                .await
                .map(Into::into),
            "BE" => company_adapter(&self.be, IdentifierScheme::BeBusinessId)?
                .lookup(value)
                .await
                .map(Into::into),
            "FI" => company_adapter(&self.fi, IdentifierScheme::FiBusinessId)?
                .lookup(value)
                .await
                .map(Into::into),
            "FR" => company_adapter(&self.fr, IdentifierScheme::FrBusinessId)?
                .lookup(value)
                .await
                .map(Into::into),
            "IS" => company_adapter(&self.is, IdentifierScheme::IsBusinessId)?
                .lookup(value)
                .await
                .map(Into::into),
            "IE" => company_adapter(&self.ie, IdentifierScheme::IeBusinessId)?
                .lookup(value)
                .await
                .map(Into::into),
            "LT" => company_adapter(&self.lt, IdentifierScheme::LtBusinessId)?
                .lookup(value)
                .await
                .map(Into::into),
            "LV" => company_adapter(&self.lv, IdentifierScheme::LvBusinessId)?
                .lookup(value)
                .await
                .map(Into::into),
            "SK" => company_adapter(&self.sk, IdentifierScheme::SkBusinessId)?
                .lookup(value)
                .await
                .map(Into::into),
            "PL" => company_adapter(&self.pl, IdentifierScheme::PlBusinessId)?
                .lookup(value)
                .await
                .map(Into::into),
            "NO" => company_adapter(&self.no, IdentifierScheme::NoBusinessId)?
                .lookup(value)
                .await
                .map(Into::into),
            "EE" => company_adapter(&self.ee, IdentifierScheme::EeBusinessId)?
                .lookup(value)
                .await
                .map(Into::into),
            "GR" => company_adapter(&self.gr, IdentifierScheme::GrBusinessId)?
                .lookup(value)
                .await
                .map(Into::into),
            "CZ" => company_adapter(&self.cz, IdentifierScheme::CzBusinessId)?
                .lookup(value)
                .await
                .map(Into::into),
            _ => Err(LookupError::from(
                crate::model::LookupFailure::UnsupportedIdentifierScheme,
            )),
        }
    }
}

fn company_adapter<T>(adapter: &Option<T>, scheme: IdentifierScheme) -> Result<&T, LookupError> {
    adapter
        .as_ref()
        .ok_or_else(|| disabled(Operation::LookupCompany, scheme))
}

#[cfg(test)]
impl Client {
    pub(crate) fn with_test_registries(bg: BgRegistryClient, ro: RoRegistryClient) -> Self {
        Self {
            bg: Some(crate::bg::BgCompanyClient::new(
                bg,
                crate::vies::ViesClient::for_test(
                    "http://127.0.0.1:1/unused".into(),
                    Default::default(),
                ),
            )),
            ro: Some(ro),
            ..Self::empty_for_test()
        }
    }
    pub(crate) fn with_test_vies(mut self, endpoint: String) -> Self {
        self.bg
            .as_mut()
            .unwrap()
            .set_test_vies(crate::vies::ViesClient::for_test(
                endpoint,
                Default::default(),
            ));
        self
    }
    pub(crate) fn for_test(endpoint: String, config: ClientConfig) -> Self {
        let mut client = Self::new().unwrap();
        client.vies = Some(crate::vies::ViesClient::for_test(endpoint, config));
        client
    }
}

#[cfg(test)]
impl Client {
    pub(crate) fn with_test_czech(cz: crate::CzRegistryClient) -> Self {
        Self {
            cz: Some(cz),
            ..Self::empty_for_test()
        }
    }
}

#[cfg(test)]
impl Client {
    pub(crate) fn with_test_austrian(at: crate::AtRegistryClient) -> Self {
        Self {
            at: Some(at),
            ..Self::empty_for_test()
        }
    }
}

#[cfg(test)]
impl Client {
    pub(crate) fn with_test_belgian(be: crate::BeRegistryClient) -> Self {
        Self {
            be: Some(be),
            ..Self::empty_for_test()
        }
    }
    pub(crate) fn with_test_finnish(fi: crate::FiRegistryClient) -> Self {
        Self {
            fi: Some(fi),
            ..Self::empty_for_test()
        }
    }
    pub(crate) fn with_test_french(fr: crate::FrRegistryClient) -> Self {
        Self {
            fr: Some(fr),
            ..Self::empty_for_test()
        }
    }
    pub(crate) fn with_test_icelandic(is: crate::IsRegistryClient) -> Self {
        Self {
            is: Some(is),
            ..Self::empty_for_test()
        }
    }
    pub(crate) fn with_test_irish(ie: crate::IeRegistryClient) -> Self {
        Self {
            ie: Some(ie),
            ..Self::empty_for_test()
        }
    }
    pub(crate) fn with_test_lithuanian(lt: crate::LtRegistryClient) -> Self {
        Self {
            lt: Some(lt),
            ..Self::empty_for_test()
        }
    }
    pub(crate) fn with_test_latvian(lv: crate::LvRegistryClient) -> Self {
        Self {
            lv: Some(lv),
            ..Self::empty_for_test()
        }
    }
    pub(crate) fn with_test_slovak(sk: crate::SkRegistryClient) -> Self {
        Self {
            sk: Some(sk),
            ..Self::empty_for_test()
        }
    }
    pub(crate) fn with_test_polish(pl: crate::PlRegistryClient) -> Self {
        Self {
            pl: Some(pl),
            ..Self::empty_for_test()
        }
    }
    pub(crate) fn with_test_norwegian(no: crate::NoRegistryClient) -> Self {
        Self {
            no: Some(no),
            ..Self::empty_for_test()
        }
    }
    pub(crate) fn with_test_estonian(ee: crate::EeRegistryClient) -> Self {
        Self {
            ee: Some(ee),
            ..Self::empty_for_test()
        }
    }
}

#[cfg(test)]
impl Client {
    pub(crate) fn with_test_greek(gr: crate::GrRegistryClient) -> Self {
        Self {
            gr: Some(gr),
            ..Self::empty_for_test()
        }
    }
}

#[cfg(test)]
impl Client {
    fn empty_for_test() -> Self {
        // The empty builder constructs no transports, including the BG VIES dependency.
        ClientBuilder::empty().build().unwrap()
    }
}
