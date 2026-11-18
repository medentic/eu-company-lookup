use serde::{Serialize, Serializer};

macro_rules! source_enum {
    ($name:ident, $doc:literal, {$($variant:ident => $wire:literal),+ $(,)?}) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq)]
        #[non_exhaustive]
        pub enum $name {
            $(#[doc = $wire] $variant,)+
            /// Unrecognized source value retained without interpretation.
            Other(String),
        }
        impl $name {
            /// Stable JSON representation or unchanged source fallback.
            pub fn as_str(&self) -> &str { match self { $(Self::$variant => $wire,)+ Self::Other(value) => value } }
        }
        impl std::ops::Deref for $name { type Target = str; fn deref(&self) -> &str { self.as_str() } }
        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> { serializer.serialize_str(self.as_str()) }
        }
    }
}
/// Shared normalized company lifecycle across registry results, distinct from tax status.
/// Public lookups require a resolved variant; unresolved pure projections use
/// `Unknown` and are rejected with `unresolved_data`. Raw values live
/// in separate source fields. Absent lifecycle evidence is represented by `None`; documented cessation dates may establish closure.
///
/// Arbitrary strings cannot be constructed as a company status:
/// ```compile_fail
/// use eu_company_lookup::CompanyStatus;
/// let status = CompanyStatus::Other("custom status".into());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum CompanyStatus {
    /// The source explicitly identifies an active company/record.
    Active,
    /// The source explicitly identifies a closed or deleted company/record.
    Closed,
    /// Explicit source state: liquidation.
    Liquidation,
    /// Explicit source state: bankruptcy.
    Bankruptcy,
    /// Explicit source state: receivership.
    Receivership,
    /// Explicit source state: liquidationAndReceivership.
    LiquidationAndReceivership,
    /// Explicit source state: restructuring.
    Restructuring,
    /// Explicit source state: pendingClosure.
    PendingClosure,
    /// Explicit source state: liquidationStayed.
    LiquidationStayed,
    /// Explicit source state: administration.
    Administration,
    /// Explicit source state: inactive.
    Inactive,
    /// A source status exists but has no verified lifecycle mapping.
    Unknown,
}
impl CompanyStatus {
    /// Stable normalized JSON value, never provider text.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Closed => "closed",
            Self::Liquidation => "liquidation",
            Self::Bankruptcy => "bankruptcy",
            Self::Receivership => "receivership",
            Self::LiquidationAndReceivership => "liquidationAndReceivership",
            Self::Restructuring => "restructuring",
            Self::PendingClosure => "pendingClosure",
            Self::LiquidationStayed => "liquidationStayed",
            Self::Administration => "administration",
            Self::Inactive => "inactive",
            Self::Unknown => "unknown",
        }
    }
}
impl std::ops::Deref for CompanyStatus {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}
source_enum!(LegalForm, "Known Bulgarian legal forms with lossless source fallback.", { Et => "ЕТ", Eood => "ЕООД", Ood => "ООД", Ad => "АД", Ead => "ЕАД", Sd => "СД" });

/// Registry role supplied by a supported field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum RegistryRole {
    /// Manager in field 7.
    Manager,
    /// Sole capital owner in field 23.
    SoleCapitalOwner,
}
/// Category of a registry document reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum RegistryDocumentType {
    /// Current constitution/articles referenced by the source field.
    ConstitutiveDocument,
}
