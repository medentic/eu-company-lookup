use crate::LookupError;
use serde::{Deserialize, Deserializer, Serialize, de::Error};
use std::{fmt, str::FromStr};

/// Czech economic subject identifier (IČO), without a VAT prefix.
/// Syntax only: exactly eight ASCII digits; no checksum or registration claim.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct CzBusinessId(String);

impl CzBusinessId {
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

impl FromStr for CzBusinessId {
    type Err = LookupError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}
impl fmt::Display for CzBusinessId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for CzBusinessId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::parse(&String::deserialize(d)?).map_err(D::Error::custom)
    }
}

/// Offline IČO format check; no checksum or registration claim.
pub fn validate_cz_business_id_syntax(value: &str) -> Result<CzBusinessId, LookupError> {
    CzBusinessId::parse(value)
}

impl AsRef<str> for CzBusinessId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl TryFrom<&str> for CzBusinessId {
    type Error = LookupError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for CzBusinessId {
    type Error = LookupError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}
impl From<CzBusinessId> for String {
    fn from(value: CzBusinessId) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_zeroes_and_rejects_other_schemes() {
        assert_eq!(
            CzBusinessId::parse(" 00123456 ").unwrap().as_str(),
            "00123456"
        );
        for invalid in ["CZ00123456", "1234567", "123456789", "１２３４５６７８", ""] {
            assert!(CzBusinessId::parse(invalid).is_err());
        }
    }
    #[test]
    fn deserialization_uses_validation() {
        assert!(serde_json::from_str::<CzBusinessId>("\"CZ00123456\"").is_err());
        let id: CzBusinessId = serde_json::from_str("\"00123456\"").unwrap();
        assert_eq!(serde_json::to_string(&id).unwrap(), "\"00123456\"");
    }
}
