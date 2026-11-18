//! Conservative request preflight; identifier parsing remains syntax-only.
mod rules;
use super::VatId;
use crate::{LookupError, model::LookupFailure};

/// Offline checksum outcome for a syntax-checked VAT identifier.
/// Neither a passed nor an unsupported check establishes VAT registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum VatChecksumResult {
    /// The identifier satisfies a supported checksum rule.
    Passed,
    /// The identifier fails a supported checksum rule.
    Failed,
    /// No checksum rule is applied to this identifier variant.
    Unsupported,
}

impl VatId {
    /// Check supported national checksum rules without network access or a client.
    /// Parsing remains syntax-only; unsupported variants are not failures.
    /// This does not verify registration, allocation or birth dates.
    ///
    /// ```
    /// use eu_company_lookup::{VatChecksumResult, VatId};
    /// let vat = VatId::parse("BG423456789")?;
    /// assert_eq!(vat.validate_checksum(), VatChecksumResult::Passed);
    /// # Ok::<(), eu_company_lookup::LookupError>(())
    /// ```
    pub fn validate_checksum(&self) -> VatChecksumResult {
        match rules::check(self.prefix(), self.number()) {
            Some(true) => VatChecksumResult::Passed,
            Some(false) => VatChecksumResult::Failed,
            None => VatChecksumResult::Unsupported,
        }
    }
}

pub(super) fn validate(vat: &VatId) -> Result<(), LookupError> {
    if vat.validate_checksum() == VatChecksumResult::Failed {
        Err(LookupError::from(LookupFailure::InvalidInput))
    } else {
        Ok(())
    }
}

#[cfg(test)]
pub(super) const EXAMPLES: &[(&str, &str)] = &[
    ("ATU82345678", "ATU82345670"),
    ("BE0123452789", "BE0123452780"),
    ("BG423456789", "BG423456780"),
    ("CY92345678A", "CY92345670A"),
    ("CZ82345678", "CZ82345670"),
    ("DE423456789", "DE423456780"),
    ("DK13345678", "DK13345670"),
    ("EE823456789", "EE823456780"),
    ("EL323456789", "EL323456780"),
    ("ESA42345678", "ESA42345670"),
    ("FI02345678", "FI02345670"),
    ("FR11123456782", "FR11123456780"),
    ("HR02345678901", "HR02345678900"),
    ("HU32345678", "HU32345670"),
    ("IE1234667A", "IE1234660A"),
    ("IT82345670010", "IT82345670011"),
    ("LT723456710", "LT723456711"),
    ("LU78345678", "LU78345670"),
    ("LV40000000000", "LV40000000001"),
    ("MT12845678", "MT12845670"),
    ("NL323456789B01", "NL323456780B01"),
    ("PL3234567890", "PL3234567891"),
    ("PT123456789", "PT123456780"),
    ("RO19", "RO10"),
    ("SE923456789001", "SE923456789101"),
    ("SI82345678", "SI82345670"),
    ("SK7234567890", "SK7234567891"),
    ("XI723456789", "XI723456780"),
];

#[cfg(test)]
mod tests {
    use super::*;

    // Synthetic seeds, cross-checked against python-stdnum 2.2. No live IDs.
    #[test]
    fn supported_country_checksums_and_corruptions() {
        let prefixes: std::collections::BTreeSet<_> = super::super::identifiers::FORMATS
            .iter()
            .map(|(prefix, _)| *prefix)
            .collect();
        assert_eq!(EXAMPLES.len(), prefixes.len());
        assert_eq!(
            EXAMPLES
                .iter()
                .map(|(good, _)| &good[..2])
                .collect::<std::collections::BTreeSet<_>>(),
            prefixes
        );
        for (good, bad) in EXAMPLES {
            assert!(validate(&VatId::parse(good).unwrap()).is_ok(), "{good}");
            // Parsing/Serde remain syntax-only, including old negative evidence.
            let vat: VatId = serde_json::from_str(&format!("\"{bad}\"")).unwrap();
            assert!(validate(&vat).is_err(), "{bad}");
        }
    }

    #[test]
    fn alternate_national_checksum_variants() {
        // Synthetic values cross-checked against python-stdnum 2.2.
        for (good, bad) in [
            ("BE1123456770", "BE1123456771"),
            ("BG9001011232", "BG9001011230"),
            ("BG9001011237", "BG9001011230"),
            ("BG9001011238", "BG9001011230"),
            ("CZ612345670", "CZ612345671"),
            ("CZ5401011231", "CZ5401011232"),
            ("ES14345678A", "ES14345678B"),
            ("ESX6234567A", "ESX6234567B"),
            ("IE1A27456A", "IE1A27456B"),
            ("IE1*27456A", "IE1*27456B"),
            ("IE1+27456A", "IE1+27456B"),
            ("IE9234567AA", "IE9234567BA"),
            ("LT023456789010", "LT023456789011"),
            ("NL123456789B13", "NL123456789B14"),
            ("XI723456789001", "XI723456780001"),
        ] {
            assert!(validate(&VatId::parse(good).unwrap()).is_ok(), "{good}");
            assert!(validate(&VatId::parse(bad).unwrap()).is_err(), "{bad}");
        }
    }

    #[test]
    fn unsupported_variants_are_not_rejected_as_bad_checksums() {
        for raw in [
            "FRAB123456789",
            "LV32000000000",
            "XIGD001",
            "XIHA500",
            "CZ530101123",
            "IT12345671010",
        ] {
            assert!(validate(&VatId::parse(raw).unwrap()).is_ok(), "{raw}");
        }
    }

    #[test]
    fn french_vat_key_does_not_require_a_separate_siren_checksum() {
        // Numeric key over synthetic SIREN 123456789, whose Luhn check fails.
        assert!(validate(&VatId::parse("FR32123456789").unwrap()).is_ok());
        assert!(validate(&VatId::parse("FR33123456789").unwrap()).is_err());
    }
    #[test]
    #[ignore = "requires local-only saved VIES corpus; no network"]
    fn saved_valid_observations_pass_checksum_preflight() {
        let root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/vies/fixtures/countries");
        let mut valid = 0;
        let mut negative = 0;
        for entry in std::fs::read_dir(root).expect("local corpus not installed") {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|ext| ext != "json") {
                continue;
            }
            let cases: Vec<serde_json::Value> =
                serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
            for case in cases {
                let vat = VatId::parse(case["vatId"].as_str().unwrap()).unwrap();
                if case["valid"] == true {
                    assert!(
                        validate(&vat).is_ok(),
                        "saved valid observation rejected: {vat}"
                    );
                    valid += 1;
                } else {
                    // Historical matched negatives need only remain parseable.
                    negative += 1;
                }
            }
        }
        assert!(valid > 0);
        println!("{valid} valid and {negative} negative saved observations preserved");
    }
}
