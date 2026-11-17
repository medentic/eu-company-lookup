use crate::LookupError;
use serde::{Deserialize, Deserializer, Serialize, de::Error};
use std::{fmt, str::FromStr};

/// Icelandic national business identifier.
/// Syntax only: ten ASCII digits; no checksum or registration claim.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct IsBusinessId(String);

impl IsBusinessId {
    /// Remove ASCII whitespace and check the supported business-code format.
    /// VAT-prefixed and personal tax identifiers are rejected.
    pub fn parse(value: &str) -> Result<Self, LookupError> {
        if value.len() > 64 || !value.is_ascii() {
            return Err(LookupError::InvalidInput);
        }
        let value: String = value.chars().filter(|c| !c.is_ascii_whitespace()).collect();
        if value.len() != 10 || !value.bytes().all(|b| b.is_ascii_digit()) {
            return Err(LookupError::InvalidInput);
        }
        let day = value[..2]
            .parse::<u8>()
            .map_err(|_| LookupError::InvalidInput)?;
        if !(41..=71).contains(&day) {
            return Err(LookupError::InvalidInput);
        }
        Ok(Self(value))
    }

    /// Normalized code with leading zeroes retained.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for IsBusinessId {
    type Err = LookupError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}
impl fmt::Display for IsBusinessId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for IsBusinessId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::parse(&String::deserialize(d)?).map_err(D::Error::custom)
    }
}

/// Offline Icelandic business-ID format check; no checksum or registration claim.
pub fn validate_is_business_id_syntax(value: &str) -> Result<IsBusinessId, LookupError> {
    IsBusinessId::parse(value)
}

impl AsRef<str> for IsBusinessId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl TryFrom<&str> for IsBusinessId {
    type Error = LookupError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for IsBusinessId {
    type Error = LookupError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}
impl From<IsBusinessId> for String {
    fn from(value: IsBusinessId) -> Self {
        value.0
    }
}
