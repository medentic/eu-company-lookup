//! Development report semantics, exercised against cross-language contract fixtures.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Status {
    Pending,
    ProviderError,
    NeedsReview,
    ValidityChanged,
    SourceChanged,
    ParserMismatch,
    Ambiguous,
    Passed,
}
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub(crate) struct Facts {
    pub observed: bool,
    pub response: bool,
    pub expectation: bool,
    pub validity_changed: bool,
    pub source_changed: bool,
    pub parser_mismatch: bool,
    pub ambiguous: bool,
}
pub(crate) fn classify(f: &Facts) -> Status {
    if !f.observed {
        Status::Pending
    } else if !f.response {
        Status::ProviderError
    } else if !f.expectation {
        Status::NeedsReview
    } else if f.validity_changed {
        Status::ValidityChanged
    } else if f.source_changed {
        Status::SourceChanged
    } else if f.parser_mismatch {
        Status::ParserMismatch
    } else if f.ambiguous {
        Status::Ambiguous
    } else {
        Status::Passed
    }
}

#[test]
fn shared_report_contract() {
    #[derive(Deserialize)]
    struct Case {
        facts: Facts,
        status: Status,
    }
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("fixtures/report-contract.json")).unwrap();
    for case in cases {
        assert_eq!(classify(&case.facts), case.status);
    }
}
