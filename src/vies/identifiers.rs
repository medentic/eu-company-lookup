use crate::LookupError;
use regex_lite::Regex;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr, sync::LazyLock};

// Conservative national format checks; deliberately not checksum algorithms.
// Sources and limitations are documented in README.
pub(super) const FORMATS: &[(&str, &str)] = &[
    ("AT", "U[0-9]{8}"),
    ("BE", "[01][0-9]{9}"),
    ("BG", "[0-9]{9,10}"),
    ("CY", "[0-9]{8}[A-Z]"),
    ("CZ", "[0-9]{8,10}"),
    ("DE", "[0-9]{9}"),
    ("DK", "[0-9]{8}"),
    ("EE", "[0-9]{9}"),
    ("EL", "[0-9]{9}"),
    ("ES", "([A-Z][0-9]{7}[A-Z0-9]|[0-9]{8}[A-Z])"),
    ("FI", "[0-9]{8}"),
    ("FR", "[A-Z0-9]{2}[0-9]{9}"),
    ("HR", "[0-9]{11}"),
    ("HU", "[0-9]{8}"),
    ("IE", "([0-9]{7}[A-Z]{1,2}|[0-9][A-Z*+][0-9]{5}[A-Z])"),
    ("IT", "[0-9]{11}"),
    ("LT", "([0-9]{9}|[0-9]{12})"),
    ("LU", "[0-9]{8}"),
    ("LV", "[0-9]{11}"),
    ("MT", "[0-9]{8}"),
    ("NL", "[0-9]{9}B[0-9]{2}"),
    ("PL", "[0-9]{10}"),
    ("PT", "[0-9]{9}"),
    ("RO", "[0-9]{2,10}"),
    ("SE", "[0-9]{12}"),
    ("SI", "[0-9]{8}"),
    ("SK", "[0-9]{10}"),
    ("XI", "([0-9]{9}|[0-9]{12}|GD[0-9]{3}|HA[0-9]{3})"),
];

static PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    FORMATS
        .iter()
        .map(|(_, pattern)| Regex::new(&format!("^{pattern}$")).expect("static VAT format"))
        .collect()
});

/// Syntax-checked VAT ID. Creation and deserialization both validate input.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct VatId(String);

/// Normalize ASCII letter case and remove ASCII spaces/whitespace, then check
/// the national format. Does not check checksums or query a registry.
/// Raw input is bounded at 64 bytes. Punctuation is not stripped.
pub fn validate_vat_syntax(raw: &str) -> Result<VatId, LookupError> {
    if raw.len() > 64 || !raw.is_ascii() {
        return Err(LookupError::InvalidInput);
    }
    let value: String = raw
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .map(|b| char::from(b.to_ascii_uppercase()))
        .collect();
    if !(4..=14).contains(&value.len())
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'*' | b'+'))
        || !value.as_bytes()[..2].iter().all(u8::is_ascii_uppercase)
    {
        return Err(LookupError::InvalidInput);
    }
    let index = FORMATS
        .iter()
        .position(|(prefix, _)| *prefix == &value[..2])
        .ok_or(LookupError::UnsupportedJurisdiction)?;
    if !PATTERNS[index].is_match(&value[2..]) {
        return Err(LookupError::InvalidInput);
    }
    Ok(VatId(value))
}

impl VatId {
    /// Parse using the same normalization as [`validate_vat_syntax`].
    pub fn parse(raw: &str) -> Result<Self, LookupError> {
        validate_vat_syntax(raw)
    }
    /// VIES prefix; EL is distinct from ISO country code GR.
    pub fn prefix(&self) -> &str {
        &self.0[..2]
    }
    /// Number without the VIES prefix, preserving leading zeroes.
    pub fn number(&self) -> &str {
        &self.0[2..]
    }
    /// Complete normalized VAT identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Display for VatId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
impl FromStr for VatId {
    type Err = LookupError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}
impl TryFrom<String> for VatId {
    type Error = LookupError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}
impl From<VatId> for String {
    fn from(value: VatId) -> Self {
        value.0
    }
}
