use crate::{
    BgRegistryClient, BgRegistryConfig, BusinessId, ClientConfig, CompanyLookupResult,
    IdentifierScheme, JurisdictionCapabilities, LookupError, Operation, Provider, RoRegistryClient,
    RoRegistryConfig, VatLookupResult,
};

/// Provider-independent lookup facade. Clones share each enabled adapter's
/// admission state. No automatic retry or cross-provider fallback is introduced.
#[derive(Clone, Debug)]
pub struct Client {
    vies: Option<crate::vies::ViesClient>,
    bg: Option<BgRegistryClient>,
    ro: Option<RoRegistryClient>,
}

/// Named adapter configuration. Defaults enable all implemented lookup adapters;
/// use `empty()` to explicitly select only the adapters your application needs.
#[derive(Clone, Debug)]
pub struct ClientBuilder {
    vies: Option<ClientConfig>,
    bg: Option<BgRegistryConfig>,
    ro: Option<RoRegistryConfig>,
}
impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            vies: Some(Default::default()),
            bg: Some(Default::default()),
            ro: Some(Default::default()),
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
        }
    }
    /// Enable VIES with these transport/admission bounds.
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
    /// Disable VIES without affecting country registry clients.
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
        Ok(Client {
            vies: self
                .vies
                .map(crate::vies::ViesClient::with_config)
                .transpose()?,
            bg: self.bg.map(BgRegistryClient::with_config).transpose()?,
            ro: self.ro.map(RoRegistryClient::with_config).transpose()?,
        })
    }
}

fn disabled(operation: Operation, identifier_scheme: IdentifierScheme) -> LookupError {
    LookupError::OperationDisabled {
        operation,
        identifier_scheme,
    }
}

impl Client {
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
    /// not establish lookup support. Use the free `capabilities()` for compiled support.
    pub fn capabilities(&self) -> Vec<JurisdictionCapabilities> {
        let mut catalog = crate::capabilities();
        for country in &mut catalog {
            country.providers.retain(|p| match p.provider {
                Provider::Vies => self.vies.is_some(),
                Provider::BulgarianCommercialRegister | Provider::Bulstat => self.bg.is_some(),
                Provider::RomanianAnaf => self.ro.is_some(),
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
    /// Lookup a national identifier through the corresponding enabled adapter.
    pub async fn lookup_company(
        &self,
        id: &BusinessId,
    ) -> Result<CompanyLookupResult, LookupError> {
        match id.scheme() {
            IdentifierScheme::BgBusinessId => self
                .bg
                .as_ref()
                .ok_or_else(|| disabled(Operation::LookupCompany, id.scheme()))?
                .lookup(id.as_str())
                .await
                .map(Into::into),
            IdentifierScheme::RoBusinessId => self
                .ro
                .as_ref()
                .ok_or_else(|| disabled(Operation::LookupCompany, id.scheme()))?
                .lookup(id.as_str())
                .await
                .map(Into::into),
            _ => Err(LookupError::UnsupportedIdentifierScheme),
        }
    }
    /// Bulgarian lookup with definitive-miss-only BULSTAT fallback.
    pub async fn lookup_bg_company(
        &self,
        value: &str,
    ) -> Result<crate::BgCompanyLookupResult, LookupError> {
        self.bg
            .as_ref()
            .ok_or_else(|| disabled(Operation::LookupBgCompany, IdentifierScheme::BgBusinessId))?
            .lookup(value)
            .await
    }
    /// Romanian fiscal lookup for today's date in Europe/Bucharest.
    pub async fn lookup_ro_company(
        &self,
        value: &str,
    ) -> Result<crate::RoCompanyLookupResult, LookupError> {
        self.ro
            .as_ref()
            .ok_or_else(|| disabled(Operation::LookupRoCompany, IdentifierScheme::RoBusinessId))?
            .lookup(value)
            .await
    }
    /// Romanian fiscal lookup for an explicit date; not a historical company extract.
    pub async fn lookup_ro_company_at(
        &self,
        value: &str,
        date: chrono::NaiveDate,
    ) -> Result<crate::RoCompanyLookupResult, LookupError> {
        self.ro
            .as_ref()
            .ok_or_else(|| disabled(Operation::LookupRoCompanyAt, IdentifierScheme::RoBusinessId))?
            .lookup_at(value, date)
            .await
    }
}

#[cfg(test)]
impl Client {
    pub(crate) fn with_test_registries(bg: BgRegistryClient, ro: RoRegistryClient) -> Self {
        Self {
            vies: None,
            bg: Some(bg),
            ro: Some(ro),
        }
    }
    pub(crate) fn for_test(endpoint: String, config: ClientConfig) -> Self {
        let mut client = Self::new().unwrap();
        client.vies = Some(crate::vies::ViesClient::for_test(endpoint, config));
        client
    }
}
