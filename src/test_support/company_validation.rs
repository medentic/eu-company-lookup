//! Evidence qualification is stricter than reproducing a parser's nullable output.
use serde_json::{Value, json};

pub(crate) fn classify(actual: &Value, reviewed_fields: &Value) -> &'static str {
    if !matches!(
        actual["status"].as_str(),
        Some(
            "active"
                | "closed"
                | "liquidation"
                | "bankruptcy"
                | "receivership"
                | "liquidationAndReceivership"
                | "restructuring"
                | "pendingClosure"
                | "liquidationStayed"
                | "administration"
                | "inactive"
        )
    ) {
        return "unresolved";
    }
    if reviewed_fields.get("/status") != Some(&actual["status"]) {
        return "needsReview";
    }
    "verifiedPositive"
}

pub(crate) fn summary(rows: &[Value], selected: &[String]) -> Value {
    let selected_set: std::collections::HashSet<_> = selected.iter().map(String::as_str).collect();
    let observed: std::collections::HashSet<_> =
        rows.iter().filter_map(|r| r["id"].as_str()).collect();
    let missing: Vec<_> = selected
        .iter()
        .filter(|id| !observed.contains(id.as_str()))
        .collect();
    let unexpected: Vec<_> = observed
        .iter()
        .filter(|id| !selected_set.contains(**id))
        .copied()
        .collect();
    let selection_matches = !selected.is_empty()
        && selected_set.len() == selected.len()
        && missing.is_empty()
        && unexpected.is_empty()
        && rows.len() == selected.len();
    let mut verified = 0;
    let mut negatives = 0;
    let mut seen = std::collections::HashSet::new();
    let mut unresolved = 0;
    let mut needs_review = 0;
    for row in rows {
        match row["validationStatus"].as_str() {
            Some("verifiedPositive" | "verifiedNegative") => {
                if row["id"]
                    .as_str()
                    .is_some_and(|id| !id.is_empty() && seen.insert(id))
                {
                    if row["validationStatus"] == "verifiedPositive" {
                        verified += 1;
                    } else {
                        negatives += 1;
                    }
                } else {
                    needs_review += 1;
                }
            }
            Some("unresolved") => unresolved += 1,
            _ => needs_review += 1,
        }
    }
    json!({"selected": selected.len(), "observed": rows.len(), "missingIds": missing, "unexpectedIds": unexpected, "batchComplete": selection_matches, "verifiedPositive": verified, "verifiedNegative": negatives,
        "unresolved": unresolved, "needsReview": needs_review,
        "validationComplete": selection_matches && verified + negatives == rows.len()})
}

#[test]
fn missing_or_unknown_lifecycle_never_qualifies_as_a_negative() {
    for status in [Value::Null, json!("unknown"), json!("")] {
        let actual = json!({"status":status});
        assert_eq!(classify(&actual, &json!({"/status":status})), "unresolved");
    }
    assert_eq!(classify(&json!({}), &json!({})), "unresolved");
    for status in ["active", "closed"] {
        let actual = json!({"status":status});
        assert_eq!(
            classify(&actual, &json!({"/status":status})),
            "verifiedPositive"
        );
        assert_eq!(
            classify(&actual, &json!({"/companyName":"Example"})),
            "needsReview"
        );
    }
    let result = summary(
        &[json!({"validationStatus":"unresolved"})],
        &["00123456".into()],
    );
    assert_eq!(result["verifiedNegative"], 0);
    assert_eq!(result["validationComplete"], false);
    assert_eq!(summary(&[], &[])["validationComplete"], false);
}

#[test]
fn definitive_negatives_are_distinct_and_duplicate_cases_do_not_complete_validation() {
    let row = json!({"id":"00123456", "validationStatus":"verifiedNegative"});
    assert_eq!(
        summary(std::slice::from_ref(&row), &["00123456".into()])["verifiedNegative"],
        1
    );
    assert_eq!(
        summary(std::slice::from_ref(&row), &["00123456".into()])["validationComplete"],
        true
    );
    assert_eq!(
        summary(&[row.clone(), row], &["00123456".into()])["validationComplete"],
        false
    );
}

pub(crate) fn unresolved_detail(actual: &Value) -> Value {
    let native = actual
        .get("nativeRegistryStatus")
        .unwrap_or(&actual["registryDetails"]["data"]["nativeRegistryStatus"]);
    json!({"source":actual["source"], "nativeRegistryStatus":native,
        "reason":if actual["status"] != "unknown" && (native.is_null() || native.as_str().is_some_and(|s| s.trim().is_empty())) {
            "missingSourceStatus"
        } else { "unmappedSourceStatus" }})
}

#[test]
fn completion_requires_the_entire_original_selection() {
    let row = json!({"id":"001", "validationStatus":"verifiedPositive"});
    let selected = vec!["001".into(), "002".into()];
    let partial = summary(std::slice::from_ref(&row), &selected);
    assert_eq!(partial["batchComplete"], false);
    assert_eq!(partial["missingIds"], json!(["002"]));
    assert_eq!(partial["validationComplete"], false);
    let negative = json!({"id":"002","validationStatus":"verifiedNegative"});
    assert_eq!(
        summary(&[row.clone(), negative], &selected)["validationComplete"],
        true
    );
    assert_eq!(
        summary(std::slice::from_ref(&row), &["999".into()])["validationComplete"],
        false
    );
    assert_eq!(
        summary(&[row], &["001".into(), "001".into()])["validationComplete"],
        false
    );
}

#[test]
fn unresolved_report_preserves_native_evidence() {
    assert_eq!(
        unresolved_detail(&json!({}))["reason"],
        "missingSourceStatus"
    );
    let detail = unresolved_detail(&json!({"source":"example","nativeRegistryStatus":"Pending"}));
    assert_eq!(detail["reason"], "unmappedSourceStatus");
    assert_eq!(detail["nativeRegistryStatus"], "Pending");
}
