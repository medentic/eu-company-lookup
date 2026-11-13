//! Typed journal schema. Historical unversioned JSON is adapted only on input.
use crate::{LimitScope, LookupError, VatId};
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", content = "detail", rename_all = "camelCase")]
pub enum AttemptError {
    TransportTimeout,
    TransportError,
    BodyTooLarge,
    BodyReadError,
    InvalidUtf8,
    TotalDeadlineExceeded,
    HttpError,
    GlobalRateLimit,
    MemberStateRateLimit,
    ServiceUnavailable,
    MemberStateUnavailable,
    ProviderTimeout,
    RejectedInput,
    UnknownSoapFault,
    MalformedResponse,
    MismatchedResponse,
    Unexpected(String),
    Legacy(String),
}
impl From<LookupError> for AttemptError {
    fn from(error: LookupError) -> Self {
        match error {
            LookupError::RateLimited {
                scope: LimitScope::Global,
                ..
            } => Self::GlobalRateLimit,
            LookupError::RateLimited {
                scope: LimitScope::MemberState,
                ..
            } => Self::MemberStateRateLimit,
            LookupError::ServiceUnavailable { .. } => Self::ServiceUnavailable,
            LookupError::MemberStateUnavailable { .. } => Self::MemberStateUnavailable,
            LookupError::ProviderTimeout { .. } => Self::ProviderTimeout,
            LookupError::ProviderRejectedInput => Self::RejectedInput,
            LookupError::UnknownSoapFault => Self::UnknownSoapFault,
            LookupError::MalformedResponse => Self::MalformedResponse,
            LookupError::MismatchedResponse => Self::MismatchedResponse,
            other => Self::Unexpected(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObservedAddress {
    pub full_address_by_vies: Option<String>,
    pub address: Option<String>,
    pub address2: Option<String>,
    pub city: Option<String>,
    pub postcode: Option<String>,
    pub region: Option<String>,
    pub country: Option<String>,
    #[serde(rename = "countryISO2")]
    pub country_iso2: Option<String>,
}
impl From<crate::VatAddress> for ObservedAddress {
    fn from(value: crate::VatAddress) -> Self {
        let a = value.fields;
        Self {
            full_address_by_vies: value.full_address_by_vies,
            address: a.address,
            address2: a.address2,
            city: a.city,
            postcode: a.postcode,
            region: a.region,
            country: a.country,
            country_iso2: a.country_iso2,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Source {
    Vies,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObservedCompany {
    pub vat_id: VatId,
    pub valid: bool,
    pub source: Source,
    pub company: Option<String>,
    pub address: ObservedAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Outcome {
    Success {
        response: Box<ObservedCompany>,
    },
    Failure {
        error: AttemptError,
    },
    Skipped {
        #[serde(rename = "bodySkipped")]
        body_skipped: bool,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attempt {
    pub attempt: usize,
    pub started_at_ms: u64,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
    pub retry_after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_body: Option<String>,
    #[serde(flatten)]
    pub outcome: Outcome,
}
impl Attempt {
    pub fn new(outcome: Outcome) -> Self {
        Self {
            attempt: 0,
            started_at_ms: 0,
            duration_ms: 0,
            http_status: None,
            retry_after: None,
            raw_body: None,
            outcome,
        }
    }
    pub fn failure(error: AttemptError) -> Self {
        Self::new(Outcome::Failure { error })
    }
    pub fn success(&self) -> bool {
        matches!(self.outcome, Outcome::Success { .. })
    }
    pub fn retryable(&self) -> bool {
        self.http_status == Some(502) && self.retry_after.is_none()
    }
    #[cfg(test)]
    pub fn status(status: u16) -> Self {
        let mut row = Self::new(Outcome::Skipped { body_skipped: true });
        row.http_status = Some(status);
        row
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub total_ms: u64,
    pub attempts: Vec<Attempt>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    pub schema_version: u32,
    pub vat_id: VatId,
    pub checked_at: chrono::DateTime<chrono::Utc>,
    #[serde(flatten)]
    pub run: Run,
}
impl Observation {
    pub fn last(&self) -> &Attempt {
        self.run
            .attempts
            .last()
            .expect("validated nonempty attempts")
    }
    pub fn from_json(line: &str) -> Result<Self, String> {
        let mut value: serde_json::Value = serde_json::from_str(line).map_err(|e| e.to_string())?;
        let legacy = value.get("schemaVersion").is_none();
        if legacy {
            value["schemaVersion"] = SCHEMA_VERSION.into();
        }
        if value["schemaVersion"] != SCHEMA_VERSION {
            return Err("unsupported journal schema".into());
        }
        let attempts = value["attempts"].as_array_mut().ok_or("missing attempts")?;
        if attempts.is_empty() || attempts.len() > 3 {
            return Err("expected one to three attempts".into());
        }
        for attempt in attempts {
            let count = ["response", "error", "bodySkipped"]
                .iter()
                .filter(|key| !attempt[**key].is_null())
                .count();
            if count != 1 {
                return Err("attempt must have exactly one outcome".into());
            }
            if legacy && let Some(error) = attempt["error"].as_str() {
                attempt["error"] =
                    serde_json::to_value(AttemptError::Legacy(error.into())).unwrap();
            }
        }
        let row: Self = serde_json::from_value(value).map_err(|e| e.to_string())?;
        for (index, attempt) in row.run.attempts.iter().enumerate() {
            if matches!(
                attempt.outcome,
                Outcome::Skipped {
                    body_skipped: false
                }
            ) {
                return Err("skipped-body outcome must be true".into());
            }
            if attempt
                .http_status
                .is_some_and(|s| !(100..=599).contains(&s))
            {
                return Err("invalid HTTP status".into());
            }
            if attempt.started_at_ms.saturating_add(attempt.duration_ms) > row.run.total_ms {
                return Err("attempt exceeds recorded lookup duration".into());
            }
            if attempt.attempt != index + 1 {
                return Err("nonsequential attempt number".into());
            }
            if let Outcome::Success { response } = &attempt.outcome
                && response.vat_id != row.vat_id
            {
                return Err("response ID differs from observation".into());
            }
        }
        Ok(row)
    }
}

#[test]
fn typed_records_round_trip_and_reject_conflicting_outcomes() {
    let row = Observation {
        schema_version: SCHEMA_VERSION,
        vat_id: VatId::parse("BG123456789").unwrap(),
        checked_at: "2026-11-13T10:00:00Z".parse().unwrap(),
        run: Run {
            total_ms: 12,
            attempts: vec![Attempt {
                attempt: 1,
                duration_ms: 12,
                ..Attempt::failure(AttemptError::MemberStateRateLimit)
            }],
        },
    };
    let text = serde_json::to_string(&row).unwrap();
    let parsed = Observation::from_json(&text).unwrap();
    assert!(matches!(
        parsed.last().outcome,
        Outcome::Failure {
            error: AttemptError::MemberStateRateLimit
        }
    ));
    let mut conflicting: serde_json::Value = serde_json::from_str(&text).unwrap();
    conflicting["attempts"][0]["bodySkipped"] = true.into();
    assert!(Observation::from_json(&conflicting.to_string()).is_err());
    conflicting["schemaVersion"] = 99.into();
    assert!(Observation::from_json(&conflicting.to_string()).is_err());
}

#[test]
fn historical_journal_errors_are_preserved_at_input_boundary() {
    let text = r#"{"vatId":"BG123456789","checkedAt":"2026-11-13T10:00:00Z","totalMs":12,"attempts":[{"attempt":1,"startedAtMs":0,"durationMs":12,"httpStatus":200,"error":"RateLimited { scope: MemberState, retry_after: None }"}]}"#;
    let row = Observation::from_json(text).unwrap();
    assert!(
        matches!(&row.last().outcome, Outcome::Failure { error: AttemptError::Legacy(text) } if text.contains("MemberState"))
    );
    assert!(Observation::from_json(&serde_json::to_string(&row).unwrap()).is_ok());
}

#[test]
#[ignore = "offline validation of local journals; set VIES_REPLAY_DIR"]
fn validate_saved_typed_journals() {
    let root = std::path::PathBuf::from(std::env::var("VIES_REPLAY_DIR").unwrap());
    let mut count = 0;
    let mut soap_bodies = 0;
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy();
        if !(name.starts_with("batch-") || name == "observations.jsonl")
            || path.extension().is_none_or(|e| e != "jsonl")
        {
            continue;
        }
        for line in std::fs::read_to_string(path).unwrap().lines() {
            let record = Observation::from_json(line).unwrap();
            let encoded = serde_json::to_string(&record).unwrap();
            let round_trip = Observation::from_json(&encoded).unwrap();
            assert_eq!(record.vat_id, round_trip.vat_id);
            assert_eq!(record.run.attempts.len(), round_trip.run.attempts.len());
            soap_bodies += validate_saved_soap(&record).expect("saved SOAP outcome changed");
            count += 1;
        }
    }
    assert!(count > 0);
    eprintln!("Validated {count} historical records and {soap_bodies} saved SOAP bodies");
}

fn validate_saved_soap(record: &Observation) -> Result<usize, &'static str> {
    let mut count = 0;
    for attempt in &record.run.attempts {
        let Some(raw) = &attempt.raw_body else {
            continue;
        };
        let successful_http = attempt.http_status.is_some_and(|s| (200..300).contains(&s));
        match (super::soap::decode(raw, &record.vat_id), &attempt.outcome) {
            (Ok(decoded), Outcome::Success { response })
                if successful_http
                    && decoded.valid == response.valid
                    && decoded.company == response.company
                    && decoded.address == response.address.full_address_by_vies => {}
            (Err(decoded), Outcome::Failure { error }) => {
                if AttemptError::from(decoded) != *error {
                    return Err("SOAP fault differs from recorded outcome");
                }
            }
            (
                Ok(_),
                Outcome::Failure {
                    error: AttemptError::HttpError,
                },
            ) if !successful_http => {}
            _ => return Err("SOAP body differs from recorded outcome"),
        }
        count += 1;
    }
    Ok(count)
}

#[test]
fn saved_soap_validation_rejects_changed_faults() {
    let mut record = Observation {
        schema_version: SCHEMA_VERSION,
        vat_id: VatId::parse("BG123456789").unwrap(),
        checked_at: "2026-11-13T10:00:00Z".parse().unwrap(),
        run: Run {
            total_ms: 1,
            attempts: vec![Attempt {
                attempt: 1,
                duration_ms: 1,
                http_status: Some(200),
                raw_body: Some(crate::vies::tests::soap::fault("MS_UNAVAILABLE")),
                ..Attempt::failure(AttemptError::MemberStateUnavailable)
            }],
        },
    };
    assert_eq!(validate_saved_soap(&record), Ok(1));
    record.run.attempts[0].raw_body = Some(crate::vies::tests::soap::fault("INVALID_INPUT"));
    assert!(validate_saved_soap(&record).is_err());
}
