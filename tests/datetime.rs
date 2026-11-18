//! Public datetime types and serialized compatibility after the Jiff migration.
use eu_company_lookup::{CivilDateTime, Date, RetryAfter, Timestamp};
use serde_json::json;

#[test]
fn exported_dates_validate_calendar_and_preserve_json_shapes() {
    let date = Date::new(2024, 2, 29).unwrap();
    assert_eq!(serde_json::to_value(date).unwrap(), json!("2024-02-29"));
    assert!(Date::new(2023, 2, 29).is_err());
    assert!("2024-02-30".parse::<Date>().is_err());
    let local: CivilDateTime = "2024-02-29T12:34:56.123".parse().unwrap();
    assert_eq!(
        serde_json::to_value(local).unwrap(),
        json!("2024-02-29T12:34:56.123")
    );
    let instant: Timestamp = "2024-02-29T14:34:56.123+02:00".parse().unwrap();
    assert_eq!(
        serde_json::to_value(instant).unwrap(),
        json!("2024-02-29T12:34:56.123Z")
    );
    assert_eq!(
        serde_json::to_value(RetryAfter::Date(instant)).unwrap(),
        json!({"Date":"2024-02-29T12:34:56.123Z"})
    );
}
