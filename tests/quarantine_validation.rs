//! The pins of slice 6, the quarantine validation, over
//! synthetic fixtures the suite writes itself and over the
//! binary's validate-quarantine door.

use std::path::PathBuf;
use std::process::Command;

use greenharvest::bigint::BigInt;
use greenharvest::ingest::{
    quarantine_document, read_quarantine_text, read_records_text, Quarantine, RecordSet, TestRecord,
};
use greenharvest::jsonvalue::JsonValue;
use greenharvest::report::read_report_text;
use greenharvest::{build_plan, build_quarantine, report_document, validate_quarantine};

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_greenharvest"))
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let path = std::env::temp_dir().join(format!(
            "greenharvest-quarantine-validation-{}-{tag}",
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

/// One suite whose plan at a budget of 15 quarantines
/// alpha and holds beta blocked on the defect they share,
/// and whose plan at a budget of 5 quarantines gamma as
/// well.
const RECORDS: &str = r#"{
  "format": "suitediet-records/1",
  "suite": "quarantine validation suite",
  "tests": [
    {"name": "alpha", "runtime_ms": 100, "caught": ["d1"]},
    {"name": "beta", "runtime_ms": 50, "caught": ["d1"]},
    {"name": "gamma", "runtime_ms": 10, "caught": ["d2"]},
    {"name": "delta", "runtime_ms": 5, "caught": ["d2"]}
  ]
}"#;

fn record_set() -> RecordSet {
    read_records_text(RECORDS).expect("the records read")
}

fn record_named(name: &str) -> TestRecord {
    record_set()
        .record_named(name)
        .expect("the record stands")
        .clone()
}

fn report_at(budget: i64) -> JsonValue {
    let plan = build_plan(&record_set(), &BigInt::from_i64(budget)).expect("the plan builds");
    report_document(&plan)
}

fn quarantine_at(budget: i64) -> Quarantine {
    let plan = build_plan(&record_set(), &BigInt::from_i64(budget)).expect("the plan builds");
    build_quarantine(&record_set(), &plan)
}

fn disagreement(report: &JsonValue, quarantine: &Quarantine) -> String {
    validate_quarantine(report, quarantine)
        .expect_err("the standing refuses")
        .to_string()
}

const DISAGREEMENT: &str = "the quarantine record disagrees with the report of its plan";

#[test]
fn test_a_genuine_quarantine_validates() {
    let report = report_at(15);
    let quarantine = quarantine_at(15);
    validate_quarantine(&report, &quarantine).expect("the genuine quarantine validates");
    let scratch = Scratch::new("genuine");
    let report_path = scratch.0.join("report.json");
    let quarantine_path = scratch.0.join("quarantine.json");
    let report_text = greenharvest::canonical_text(&report);
    let quarantine_text = greenharvest::canonical_text(&quarantine_document(&quarantine));
    std::fs::write(&report_path, &report_text).expect("the report stands");
    std::fs::write(&quarantine_path, &quarantine_text).expect("the quarantine record stands");
    let output = Command::new(binary())
        .arg("validate-quarantine")
        .arg(&report_path)
        .arg(&quarantine_path)
        .output()
        .expect("the door runs");
    assert!(
        output.status.success(),
        "the door exits 0 on a genuine quarantine record"
    );
    assert!(output.stdout.is_empty(), "the door prints nothing");
    // The documents as read from their own text validate whole.
    let read_report = read_report_text(&report_text).expect("the report reads back");
    let read_quarantine =
        read_quarantine_text(&quarantine_text).expect("the quarantine record reads back");
    validate_quarantine(&read_report, &read_quarantine).expect("the read documents validate");
}

#[test]
fn test_a_quarantine_carrying_a_blocked_test_is_refused() {
    let quarantine = Quarantine {
        suite: "quarantine validation suite".to_string(),
        tests: vec![record_named("alpha"), record_named("beta")],
    };
    assert_eq!(disagreement(&report_at(15), &quarantine), DISAGREEMENT);
}

#[test]
fn test_a_quarantine_missing_a_cleared_test_is_refused() {
    let quarantine = Quarantine {
        suite: "quarantine validation suite".to_string(),
        tests: Vec::new(),
    };
    assert_eq!(disagreement(&report_at(15), &quarantine), DISAGREEMENT);
}

#[test]
fn test_a_quarantine_with_an_altered_record_is_refused() {
    let mut altered = record_named("alpha");
    altered.runtime_ms = BigInt::from_i64(101);
    let quarantine = Quarantine {
        suite: "quarantine validation suite".to_string(),
        tests: vec![altered],
    };
    assert_eq!(disagreement(&report_at(15), &quarantine), DISAGREEMENT);
}

#[test]
fn test_a_quarantine_from_another_budget_is_refused() {
    let other = quarantine_at(5);
    assert_eq!(disagreement(&report_at(15), &other), DISAGREEMENT);
}

#[test]
fn test_a_quarantine_naming_the_wrong_suite_is_refused() {
    let genuine = quarantine_at(15);
    let quarantine = Quarantine {
        suite: "another suite".to_string(),
        tests: genuine.tests,
    };
    assert_eq!(disagreement(&report_at(15), &quarantine), DISAGREEMENT);
}

#[test]
fn test_an_empty_quarantine_validates_against_a_plan_that_cleared_nothing() {
    let report = report_at(165);
    let quarantine = quarantine_at(165);
    assert!(
        quarantine.tests.is_empty(),
        "the plan at the full budget clears nothing"
    );
    validate_quarantine(&report, &quarantine)
        .expect("the empty quarantine record validates against its plan");
}
