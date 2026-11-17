use crate::LookupError;
use serde::{Deserialize, Deserializer, Serialize, de::Error};
use std::{fmt, str::FromStr};

/// Austrian Firmenbuch number, without a label or VAT prefix.
/// Syntax only: one to six ASCII digits followed by one letter; no checksum or registration claim.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct AtBusinessId(String);

impl AtBusinessId {
    /// Remove ASCII whitespace and check the supported business-code format.
    /// VAT-prefixed and personal tax identifiers are rejected.
    pub fn parse(value: &str) -> Result<Self, LookupError> {
        if value.len() > 64 || !value.is_ascii() {
            return Err(LookupError::InvalidInput);
        }
        let value: String = value
            .chars()
            .filter(|c| !c.is_ascii_whitespace())
            .collect::<String>()
            .to_ascii_uppercase();
        if !(2..=7).contains(&value.len())
            || !value.as_bytes()[..value.len() - 1]
                .iter()
                .all(u8::is_ascii_digit)
            || !value.as_bytes()[value.len() - 1].is_ascii_alphabetic()
        {
            return Err(LookupError::InvalidInput);
        }
        Ok(Self(value))
    }

    /// Normalized code with leading zeroes retained.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for AtBusinessId {
    type Err = LookupError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}
impl fmt::Display for AtBusinessId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for AtBusinessId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::parse(&String::deserialize(d)?).map_err(D::Error::custom)
    }
}

/// Offline Firmenbuch format check; no checksum or registration claim.
pub fn validate_at_business_id_syntax(value: &str) -> Result<AtBusinessId, LookupError> {
    AtBusinessId::parse(value)
}

impl AsRef<str> for AtBusinessId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl TryFrom<&str> for AtBusinessId {
    type Error = LookupError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for AtBusinessId {
    type Error = LookupError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}
impl From<AtBusinessId> for String {
    fn from(value: AtBusinessId) -> Self {
        value.0
    }
}
