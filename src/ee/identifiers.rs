use crate::LookupError;
use serde::{Deserialize, Deserializer, Serialize, de::Error};
use std::{fmt, str::FromStr};

/// Estonian national business identifier.
/// Syntax only: eight ASCII digits; no checksum or registration claim.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct EeBusinessId(String);

impl EeBusinessId {
    /// Remove ASCII whitespace and check the supported business-code format.
    /// VAT-prefixed and personal tax identifiers are rejected.
    pub fn parse(value: &str) -> Result<Self, LookupError> {
        if value.len() > 64 || !value.is_ascii() {
            return Err(LookupError::from(crate::model::LookupFailure::InvalidInput));
        }
        let value: String = value.chars().filter(|c| !c.is_ascii_whitespace()).collect();
        if value.len() != 8 || !value.bytes().all(|b| b.is_ascii_digit()) {
            return Err(LookupError::from(crate::model::LookupFailure::InvalidInput));
        }
        Ok(Self(value))
    }

    /// Normalized code with leading zeroes retained.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for EeBusinessId {
    type Err = LookupError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}
impl fmt::Display for EeBusinessId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for EeBusinessId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::parse(&String::deserialize(d)?).map_err(D::Error::custom)
    }
}

/// Offline Estonian business-ID format check; no checksum or registration claim.
pub fn validate_ee_business_id_syntax(value: &str) -> Result<EeBusinessId, LookupError> {
    EeBusinessId::parse(value)
}

impl AsRef<str> for EeBusinessId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl TryFrom<&str> for EeBusinessId {
    type Error = LookupError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for EeBusinessId {
    type Error = LookupError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}
impl From<EeBusinessId> for String {
    fn from(value: EeBusinessId) -> Self {
        value.0
    }
}
