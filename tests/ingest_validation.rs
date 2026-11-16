//! The pins of slice 7, the ingest validation, over
//! synthetic fixtures the suite writes itself and over the
//! binary's validate-ingest door.

use std::path::PathBuf;
use std::process::Command;

use greenharvest::catchtable::read_catch_table_text;
use greenharvest::ingest::read_records_text;
use greenharvest::validate_ingest;

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_greenharvest"))
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let path = std::env::temp_dir().join(format!(
            "greenharvest-ingest-validation-{}-{tag}",
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

/// One catch table of the suite, three tests and three
/// defects, the third defect caught by no test.
const TABLE_ONE: &str = r#"{
  "format": "greenharvest-catch-table/1",
  "suite": "ingest validation suite",
  "tests": [
    {"name": "alpha", "runtime_ms": 100},
    {"name": "beta", "runtime_ms": 50},
    {"name": "gamma", "runtime_ms": 10}
  ],
  "catches": [
    {"defect": "d1", "caught_by": ["alpha", "beta"]},
    {"defect": "d2", "caught_by": ["alpha"]},
    {"defect": "d3", "caught_by": []}
  ]
}"#;

/// A second shard table of the same suite, carrying one
/// test of the first table at the same runtime and one
/// test and one defect of its own.
const TABLE_TWO: &str = r#"{
  "format": "greenharvest-catch-table/1",
  "suite": "ingest validation suite",
  "tests": [
    {"name": "alpha", "runtime_ms": 100},
    {"name": "delta", "runtime_ms": 5}
  ],
  "catches": [
    {"defect": "d4", "caught_by": ["delta", "alpha"]}
  ]
}"#;

/// The record set the first table transposes to, spelled
/// compactly with its keys shuffled and its tests standing
/// out of name order, so the pin proves the comparison
/// stands over values and not over spelling.
const RECORDS_ONE: &str = r#"{"tests": [{"caught": ["d1"], "runtime_ms": 50, "name": "beta"}, {"name": "gamma", "runtime_ms": 10, "caught": []}, {"name": "alpha", "runtime_ms": 100, "caught": ["d1", "d2"]}], "suite": "ingest validation suite", "format": "suitediet-records/1"}"#;

/// The record set the union of the two tables transposes
/// to.
const RECORDS_UNION: &str = r#"{
  "format": "suitediet-records/1",
  "suite": "ingest validation suite",
  "tests": [
    {"name": "alpha", "runtime_ms": 100, "caught": ["d1", "d2", "d4"]},
    {"name": "beta", "runtime_ms": 50, "caught": ["d1"]},
    {"name": "delta", "runtime_ms": 5, "caught": ["d4"]},
    {"name": "gamma", "runtime_ms": 10, "caught": []}
  ]
}"#;

const DISAGREEMENT: &str =
    "error: the record set disagrees with the catch table it was transposed from\n";

struct Outcome {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn run_door(args: &[&PathBuf]) -> Outcome {
    let output = Command::new(binary())
        .arg("validate-ingest")
        .args(args)
        .output()
        .expect("the binary runs");
    Outcome {
        code: output.status.code(),
        stdout: String::from_utf8(output.stdout).expect("stdout is text"),
        stderr: String::from_utf8(output.stderr).expect("stderr is text"),
    }
}

fn assert_refused(outcome: &Outcome) {
    assert_eq!(outcome.code, Some(2));
    assert_eq!(outcome.stderr, DISAGREEMENT);
    assert_eq!(outcome.stdout, "");
}

#[test]
fn test_a_genuine_record_set_validates_against_its_table() {
    let scratch = Scratch::new("genuine");
    let table = scratch.write("table.json", TABLE_ONE);
    let records = scratch.write("records.json", RECORDS_ONE);
    let outcome = run_door(&[&table, &records]);
    assert_eq!(outcome.code, Some(0));
    assert_eq!(outcome.stdout, "");
    assert_eq!(outcome.stderr, "");
    let expected = read_catch_table_text(TABLE_ONE).expect("the table reads");
    let held = read_records_text(RECORDS_ONE).expect("the records read");
    assert!(validate_ingest(&expected.record_set, &held).is_ok());
}

#[test]
fn test_a_record_set_with_an_altered_caught_list_is_refused() {
    let scratch = Scratch::new("caught");
    let table = scratch.write("table.json", TABLE_ONE);
    let altered = RECORDS_ONE.replace(
        "\"caught\": [\"d1\"], \"runtime_ms\": 50",
        "\"caught\": [], \"runtime_ms\": 50",
    );
    let records = scratch.write("records.json", &altered);
    assert_refused(&run_door(&[&table, &records]));
}

#[test]
fn test_a_record_set_with_an_altered_runtime_is_refused() {
    let scratch = Scratch::new("runtime");
    let table = scratch.write("table.json", TABLE_ONE);
    let altered = RECORDS_ONE.replace("\"runtime_ms\": 50", "\"runtime_ms\": 51");
    let records = scratch.write("records.json", &altered);
    assert_refused(&run_door(&[&table, &records]));
    let expected = read_catch_table_text(TABLE_ONE).expect("the table reads");
    let held = read_records_text(&altered).expect("the records read");
    let error = validate_ingest(&expected.record_set, &held).expect_err("the pair disagrees");
    assert_eq!(
        error.message(),
        "the record set disagrees with the catch table it was transposed from"
    );
}

#[test]
fn test_a_record_set_missing_a_test_is_refused() {
    let scratch = Scratch::new("missing");
    let table = scratch.write("table.json", TABLE_ONE);
    let altered = RECORDS_ONE.replace(
        "{\"name\": \"gamma\", \"runtime_ms\": 10, \"caught\": []}, ",
        "",
    );
    let records = scratch.write("records.json", &altered);
    assert_refused(&run_door(&[&table, &records]));
}

#[test]
fn test_a_record_set_with_an_extra_test_is_refused() {
    let scratch = Scratch::new("extra");
    let table = scratch.write("table.json", TABLE_ONE);
    let altered = RECORDS_ONE.replace(
        "\"tests\": [",
        "\"tests\": [{\"name\": \"epsilon\", \"runtime_ms\": 1, \"caught\": []}, ",
    );
    let records = scratch.write("records.json", &altered);
    assert_refused(&run_door(&[&table, &records]));
}

#[test]
fn test_a_record_set_naming_the_wrong_suite_is_refused() {
    let scratch = Scratch::new("suite");
    let table = scratch.write("table.json", TABLE_ONE);
    let altered = RECORDS_ONE.replace("ingest validation suite", "another suite");
    let records = scratch.write("records.json", &altered);
    assert_refused(&run_door(&[&table, &records]));
}

#[test]
fn test_a_record_set_validates_against_the_union_of_two_tables() {
    let scratch = Scratch::new("union");
    let one = scratch.write("one.json", TABLE_ONE);
    let two = scratch.write("two.json", TABLE_TWO);
    let records = scratch.write("records.json", RECORDS_UNION);
    let outcome = run_door(&[&one, &two, &records]);
    assert_eq!(outcome.code, Some(0));
    assert_eq!(outcome.stdout, "");
    assert_eq!(outcome.stderr, "");
}

#[test]
fn test_a_record_set_from_one_table_is_refused_against_the_union() {
    let scratch = Scratch::new("one-of-union");
    let one = scratch.write("one.json", TABLE_ONE);
    let two = scratch.write("two.json", TABLE_TWO);
    let records = scratch.write("records.json", RECORDS_ONE);
    assert_refused(&run_door(&[&one, &two, &records]));
}

#[test]
fn test_a_table_with_an_uncaught_defect_still_validates() {
    let scratch = Scratch::new("uncaught");
    let table = scratch.write("table.json", TABLE_ONE);
    let records = scratch.write("records.json", RECORDS_ONE);
    let outcome = run_door(&[&table, &records]);
    assert_eq!(outcome.code, Some(0));
    assert_eq!(outcome.stdout, "");
    assert!(!outcome.stderr.contains("uncaught"));
}
