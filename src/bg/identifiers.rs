use crate::LookupError;
use serde::{Deserialize, Deserializer, Serialize, de::Error};
use std::{fmt, str::FromStr};

/// Bulgarian business identifier (ЕИК/code under BULSTAT), without a VAT prefix.
/// Syntax only: nine or thirteen ASCII digits; no checksum or registration claim.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct BgBusinessId(String);

impl BgBusinessId {
    /// Remove ASCII whitespace and check the supported business-code format.
    /// Personal identifiers (ten digits) and prefixed VAT IDs are rejected.
    pub fn parse(value: &str) -> Result<Self, LookupError> {
        if value.len() > 64 || !value.is_ascii() {
            return Err(LookupError::InvalidInput);
        }
        let value: String = value.chars().filter(|c| !c.is_ascii_whitespace()).collect();
        if !matches!(value.len(), 9 | 13) || !value.bytes().all(|b| b.is_ascii_digit()) {
            return Err(LookupError::InvalidInput);
        }
        Ok(Self(value))
    }

    /// Normalized code with leading zeroes retained.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for BgBusinessId {
    type Err = LookupError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}
impl fmt::Display for BgBusinessId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for BgBusinessId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::parse(&String::deserialize(d)?).map_err(D::Error::custom)
    }
}

/// Offline format check shared by Commercial Register and BULSTAT lookups.
pub fn validate_bg_business_id_syntax(value: &str) -> Result<BgBusinessId, LookupError> {
    BgBusinessId::parse(value)
}

impl AsRef<str> for BgBusinessId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl TryFrom<&str> for BgBusinessId {
    type Error = LookupError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for BgBusinessId {
    type Error = LookupError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}
impl From<BgBusinessId> for String {
    fn from(value: BgBusinessId) -> Self {
        value.0
    }
}
