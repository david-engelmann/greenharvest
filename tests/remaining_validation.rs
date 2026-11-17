//! The pins of slice 9, the remaining set validation, over
//! synthetic fixtures the suite writes itself and over the
//! binary's validate-remaining door.

use std::path::PathBuf;
use std::process::Command;

use greenharvest::bigint::BigInt;
use greenharvest::ingest::{read_records_text, read_remaining_text, records_document, RecordSet};
use greenharvest::jsonvalue::JsonValue;
use greenharvest::quarantine::remaining_records;
use greenharvest::report::{report_document, validate_remaining};
use greenharvest::{build_plan, canonical_text};

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_greenharvest"))
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let path = std::env::temp_dir().join(format!(
            "greenharvest-remaining-validation-{}-{tag}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the scratch directory stands");
        Scratch(path)
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, text).expect("the fixture writes");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// One suite whose plan at a budget of 60 holds one blocked
/// test, alpha on its unique catch, and two quarantined
/// tests, beta, whose only catch gamma also catches, and
/// delta, which catches nothing. The remaining set of the
/// plan is alpha and gamma in name order.
const RECORDS: &str = r#"{
  "format": "suitediet-records/1",
  "suite": "remaining validation suite",
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

fn genuine_remaining() -> RecordSet {
    let plan = build_plan(&record_set(), &BigInt::from_i64(60)).expect("the plan builds");
    remaining_records(&record_set(), &plan)
}

fn disagreement(mutate: impl FnOnce(&mut RecordSet)) -> String {
    let mut remaining = genuine_remaining();
    mutate(&mut remaining);
    validate_remaining(&record_set(), &genuine_report(), &remaining)
        .expect_err("the tampered remaining set refuses")
        .to_string()
}

const DISAGREEMENT: &str =
    "the remaining record set disagrees with the records and the report it was planned from";

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn test_a_genuine_remaining_set_validates() {
    let remaining = genuine_remaining();
    validate_remaining(&record_set(), &genuine_report(), &remaining)
        .expect("the genuine remaining set validates");
    // The remaining set as read from its own text validates
    // whole under the standing remaining reading.
    let text = canonical_text(&records_document(&remaining));
    let read = read_remaining_text(&text).expect("the remaining set reads back");
    validate_remaining(&record_set(), &genuine_report(), &read)
        .expect("the read remaining set validates");
    let scratch = Scratch::new("genuine");
    let records_path = scratch.write("records.json", RECORDS);
    let report_path = scratch.write("report.json", &canonical_text(&genuine_report()));
    let remaining_path = scratch.write("remaining.json", &text);
    let output = Command::new(binary())
        .arg("validate-remaining")
        .arg(&records_path)
        .arg(&report_path)
        .arg(&remaining_path)
        .output()
        .expect("the door runs");
    assert!(
        output.status.success(),
        "the door exits 0 on a genuine remaining set, stderr: {}",
        stderr(&output)
    );
    assert!(output.stdout.is_empty(), "the door prints nothing");
}

#[test]
fn test_a_remaining_set_with_an_altered_runtime_is_refused() {
    let message = disagreement(|remaining| {
        let gamma = remaining
            .tests
            .iter_mut()
            .find(|test| test.name == "gamma")
            .expect("gamma stands in the remaining set");
        gamma.runtime_ms = BigInt::from_i64(11);
    });
    assert_eq!(message, DISAGREEMENT);
    // Through the door the same drift refuses at exit code 2
    // in the disagreement words.
    let mut altered = genuine_remaining();
    altered
        .tests
        .iter_mut()
        .find(|test| test.name == "gamma")
        .expect("gamma stands in the remaining set")
        .runtime_ms = BigInt::from_i64(11);
    let scratch = Scratch::new("runtime");
    let records_path = scratch.write("records.json", RECORDS);
    let report_path = scratch.write("report.json", &canonical_text(&genuine_report()));
    let remaining_path = scratch.write(
        "remaining.json",
        &canonical_text(&records_document(&altered)),
    );
    let output = Command::new(binary())
        .arg("validate-remaining")
        .arg(&records_path)
        .arg(&report_path)
        .arg(&remaining_path)
        .output()
        .expect("the door runs");
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains(DISAGREEMENT),
        "{}",
        stderr(&output)
    );
}

#[test]
fn test_a_remaining_set_with_an_altered_caught_list_is_refused() {
    let message = disagreement(|remaining| {
        let gamma = remaining
            .tests
            .iter_mut()
            .find(|test| test.name == "gamma")
            .expect("gamma stands in the remaining set");
        gamma.caught = vec!["d9".to_string()];
    });
    assert_eq!(message, DISAGREEMENT);
}

#[test]
fn test_a_remaining_set_missing_a_blocked_test_is_refused() {
    let message = disagreement(|remaining| {
        remaining.tests.retain(|test| test.name != "alpha");
    });
    assert_eq!(message, DISAGREEMENT);
}

#[test]
fn test_a_remaining_set_missing_a_survivor_is_refused() {
    let message = disagreement(|remaining| {
        remaining.tests.retain(|test| test.name != "gamma");
    });
    assert_eq!(message, DISAGREEMENT);
}

#[test]
fn test_a_remaining_set_carrying_a_quarantined_test_is_refused() {
    let message = disagreement(|remaining| {
        let beta = record_set()
            .record_named("beta")
            .expect("beta stands in the records")
            .clone();
        remaining.tests.push(beta);
    });
    assert_eq!(message, DISAGREEMENT);
}

#[test]
fn test_a_remaining_set_naming_the_wrong_suite_is_refused() {
    let message = disagreement(|remaining| {
        remaining.suite = "another suite".to_string();
    });
    assert_eq!(message, DISAGREEMENT);
}

#[test]
fn test_a_remaining_set_from_another_budget_is_refused() {
    let wider = build_plan(&record_set(), &BigInt::from_i64(160)).expect("the wider plan builds");
    let remaining = remaining_records(&record_set(), &wider);
    assert_ne!(remaining, genuine_remaining());
    let error = validate_remaining(&record_set(), &genuine_report(), &remaining)
        .expect_err("the remaining set of another budget refuses");
    assert_eq!(error.to_string(), DISAGREEMENT);
}

#[test]
fn test_an_emptied_plan_remaining_set_validates() {
    let records = read_records_text(
        r#"{
  "format": "suitediet-records/1",
  "suite": "emptied suite",
  "tests": [
    {"name": "eta", "runtime_ms": 30, "caught": []},
    {"name": "theta", "runtime_ms": 20, "caught": []}
  ]
}"#,
    )
    .expect("the records read");
    let plan = build_plan(&records, &BigInt::from_i64(10)).expect("the plan builds");
    let report = report_document(&plan);
    let remaining = remaining_records(&records, &plan);
    assert!(remaining.tests.is_empty(), "the plan cleared every test");
    validate_remaining(&records, &report, &remaining)
        .expect("the empty remaining set of an emptied plan validates");
}
