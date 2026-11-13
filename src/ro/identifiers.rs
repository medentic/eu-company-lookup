use crate::LookupError;
use serde::{Deserialize, Deserializer, Serialize, de::Error};
use std::{fmt, str::FromStr};

/// Romanian business tax identifier (CUI), without the RO VAT prefix.
/// Syntax only: two to ten ASCII digits; no checksum or registration claim.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct RoBusinessId(String);

impl RoBusinessId {
    /// Remove ASCII whitespace and check the supported business-code format.
    /// Personal identifiers (thirteen digits) and prefixed VAT IDs are rejected.
    pub fn parse(value: &str) -> Result<Self, LookupError> {
        if value.len() > 64 || !value.is_ascii() {
            return Err(LookupError::InvalidInput);
        }
        let value: String = value.chars().filter(|c| !c.is_ascii_whitespace()).collect();
        if !(2..=10).contains(&value.len()) || !value.bytes().all(|b| b.is_ascii_digit()) {
            return Err(LookupError::InvalidInput);
        }
        Ok(Self(value))
    }

    /// Normalized code with leading zeroes retained.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for RoBusinessId {
    type Err = LookupError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}
impl fmt::Display for RoBusinessId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for RoBusinessId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::parse(&String::deserialize(d)?).map_err(D::Error::custom)
    }
}

/// Offline CUI format check; no checksum or registration claim.
pub fn validate_ro_business_id_syntax(value: &str) -> Result<RoBusinessId, LookupError> {
    RoBusinessId::parse(value)
}

impl AsRef<str> for RoBusinessId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl TryFrom<&str> for RoBusinessId {
    type Error = LookupError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for RoBusinessId {
    type Error = LookupError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}
impl From<RoBusinessId> for String {
    fn from(value: RoBusinessId) -> Self {
        value.0
    }
}
