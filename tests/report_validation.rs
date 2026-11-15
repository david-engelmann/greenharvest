//! The pins of slice 5, the report validation, over
//! synthetic fixtures the suite writes itself and over the
//! binary's validate-report door.

use std::path::PathBuf;
use std::process::Command;

use greenharvest::bigint::BigInt;
use greenharvest::ingest::{read_records_text, RecordSet};
use greenharvest::jsonvalue::JsonValue;
use greenharvest::report::{read_report_text, validate_report};
use greenharvest::{build_plan, report_document};

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_greenharvest"))
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let path = std::env::temp_dir().join(format!(
            "greenharvest-report-validation-{}-{tag}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the scratch directory stands");
        Scratch(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// One suite whose plan at a budget of 60 holds one blocked
/// pack, alpha on its unique catch, and one quarantined
/// pack, delta, which catches nothing.
const RECORDS: &str = r#"{
  "format": "suitediet-records/1",
  "suite": "validation suite",
  "tests": [
    {"name": "alpha", "runtime_ms": 100, "caught": ["d1"]},
    {"name": "beta", "runtime_ms": 50, "caught": ["d2"]},
    {"name": "gamma", "runtime_ms": 10, "caught": ["d2"]},
    {"name": "delta", "runtime_ms": 500, "caught": []}
  ]
}"#;

fn record_set() -> RecordSet {
    read_records_text(RECORDS).expect("the records read")
}

fn genuine_report() -> JsonValue {
    let plan = build_plan(&record_set(), &BigInt::from_i64(60)).expect("the plan builds");
    report_document(&plan)
}

fn tampered_report(mutate: impl FnOnce(&mut Vec<(String, JsonValue)>)) -> JsonValue {
    let JsonValue::Object(mut fields) = genuine_report() else {
        panic!("the report is an object");
    };
    mutate(&mut fields);
    JsonValue::Object(fields)
}

fn field_mut<'a>(fields: &'a mut [(String, JsonValue)], name: &str) -> &'a mut JsonValue {
    &mut fields
        .iter_mut()
        .find(|(key, _)| key == name)
        .expect("the field stands")
        .1
}

fn disagreement(mutate: impl FnOnce(&mut Vec<(String, JsonValue)>)) -> String {
    let report = tampered_report(mutate);
    validate_report(&record_set(), &report)
        .expect_err("the tampered report refuses")
        .to_string()
}

const DISAGREEMENT: &str = "the report disagrees with the records it was planned from";

#[test]
fn test_a_genuine_report_validates() {
    let report = genuine_report();
    validate_report(&record_set(), &report).expect("the genuine report validates");
    let scratch = Scratch::new("genuine");
    let records_path = scratch.0.join("records.json");
    let report_path = scratch.0.join("report.json");
    std::fs::write(&records_path, RECORDS).expect("the records stand");
    let text = greenharvest::canonical_text(&report);
    std::fs::write(&report_path, &text).expect("the report stands");
    let output = Command::new(binary())
        .arg("validate-report")
        .arg(&records_path)
        .arg(&report_path)
        .output()
        .expect("the door runs");
    assert!(
        output.status.success(),
        "the door exits 0 on a genuine report"
    );
    assert!(output.stdout.is_empty(), "the door prints nothing");
    // The report as read from its own text validates whole.
    let read = read_report_text(&text).expect("the report reads back");
    validate_report(&record_set(), &read).expect("the read report validates");
}

#[test]
fn test_a_report_with_a_flipped_disposition_is_refused() {
    let message = disagreement(|fields| {
        let JsonValue::List(packs) = field_mut(fields, "removal_packs") else {
            panic!("the packs stand as a list");
        };
        for pack in packs.iter_mut() {
            if matches!(pack.get("name"), Some(JsonValue::Text(name)) if name == "delta") {
                let JsonValue::Object(pack_fields) = pack else {
                    panic!("the pack is an object");
                };
                for (key, value) in pack_fields.iter_mut() {
                    if key == "disposition" {
                        *value = JsonValue::Text("blocked".to_string());
                    }
                }
            }
        }
    });
    assert_eq!(message, DISAGREEMENT);
}

#[test]
fn test_a_report_with_a_dropped_blocking_defect_is_refused() {
    let message = disagreement(|fields| {
        let JsonValue::List(packs) = field_mut(fields, "removal_packs") else {
            panic!("the packs stand as a list");
        };
        for pack in packs.iter_mut() {
            if matches!(pack.get("name"), Some(JsonValue::Text(name)) if name == "alpha") {
                let JsonValue::Object(pack_fields) = pack else {
                    panic!("the pack is an object");
                };
                for (key, value) in pack_fields.iter_mut() {
                    if key == "blocking_defects" {
                        *value = JsonValue::List(Vec::new());
                    }
                }
            }
        }
    });
    assert_eq!(message, DISAGREEMENT);
}

#[test]
fn test_a_report_with_a_moved_total_is_refused() {
    let message = disagreement(|fields| {
        let totals = field_mut(fields, "totals");
        let JsonValue::Object(totals_fields) = totals else {
            panic!("the totals stand as an object");
        };
        for (key, value) in totals_fields.iter_mut() {
            if key == "defects_before" {
                *value = JsonValue::Int(3);
            }
        }
    });
    assert_eq!(message, DISAGREEMENT);
}

#[test]
fn test_a_report_with_two_ranks_swapped_is_refused() {
    let message = disagreement(|fields| {
        let JsonValue::List(ranking) = field_mut(fields, "ranking") else {
            panic!("the ranking stands as a list");
        };
        ranking.swap(0, 1);
    });
    assert_eq!(message, DISAGREEMENT);
}

#[test]
fn test_a_report_naming_the_wrong_budget_is_refused() {
    let message = disagreement(|fields| {
        *field_mut(fields, "budget_ms") = JsonValue::Int(160);
    });
    assert_eq!(message, DISAGREEMENT);
}

#[test]
fn test_a_report_planned_from_other_records_is_refused() {
    let other = read_records_text(
        r#"{
  "format": "suitediet-records/1",
  "suite": "validation suite",
  "tests": [
    {"name": "alpha", "runtime_ms": 101, "caught": ["d1"]},
    {"name": "beta", "runtime_ms": 50, "caught": ["d2"]},
    {"name": "gamma", "runtime_ms": 10, "caught": ["d2"]},
    {"name": "delta", "runtime_ms": 500, "caught": []}
  ]
}"#,
    )
    .expect("the other records read");
    let error = validate_report(&other, &genuine_report())
        .expect_err("the report of another suite's records refuses");
    assert_eq!(error.to_string(), DISAGREEMENT);
}
