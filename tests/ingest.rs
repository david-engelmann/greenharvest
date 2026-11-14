//! The pins of slice 3, the catch table front end, over
//! synthetic tables the suite writes itself and over the
//! binary's ingest door.

use std::path::PathBuf;
use std::process::Command;

use greenharvest::bigint::BigInt;
use greenharvest::catchtable::read_catch_table_text;
use greenharvest::ingest::{read_records_text, records_document};
use greenharvest::{build_plan, canonical_text, report_document};

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_greenharvest"))
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let path =
            std::env::temp_dir().join(format!("greenharvest-ingest-{}-{tag}", std::process::id()));
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

const TABLE: &str = r#"{
  "format": "greenharvest-catch-table/1",
  "suite": "probe",
  "tests": [
    {"name": "alpha", "runtime_ms": 100},
    {"name": "beta", "runtime_ms": 50},
    {"name": "gamma", "runtime_ms": 10}
  ],
  "catches": [
    {"defect": "d1", "caught_by": ["alpha"]},
    {"defect": "d2", "caught_by": ["alpha", "beta"]}
  ]
}"#;

#[test]
fn test_ingest_transposes_a_catch_table_into_records() {
    let scratch = Scratch::new("transpose");
    let table_path = scratch.0.join("table.json");
    std::fs::write(&table_path, TABLE).expect("the table is written");
    let out_path = scratch.0.join("records.json");
    let output = Command::new(binary())
        .arg("ingest")
        .arg(&table_path)
        .arg("--out")
        .arg(&out_path)
        .output()
        .expect("the binary runs");
    assert!(output.status.success(), "the ingest door exits 0");
    assert!(
        output.stdout.is_empty(),
        "a table whose defects all stand caught prints nothing"
    );
    let written = std::fs::read_to_string(&out_path).expect("the record set is written");
    let expected = read_records_text(
        r#"{"format": "suitediet-records/1", "suite": "probe", "tests": [
            {"name": "alpha", "runtime_ms": 100, "caught": ["d1", "d2"]},
            {"name": "beta", "runtime_ms": 50, "caught": ["d2"]},
            {"name": "gamma", "runtime_ms": 10, "caught": []}
        ]}"#,
    )
    .expect("the hand written record set reads");
    assert_eq!(written, canonical_text(&records_document(&expected)));
}

#[test]
fn test_a_defect_caught_by_no_test_is_named_and_stands_nowhere() {
    let table = read_catch_table_text(
        r#"{"format": "greenharvest-catch-table/1", "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 100}],
            "catches": [{"defect": "d9", "caught_by": []},
                        {"defect": "d1", "caught_by": ["alpha"]}]}"#,
    )
    .expect("the table reads");
    assert_eq!(table.uncaught, vec!["d9".to_string()]);
    assert!(
        table
            .record_set
            .tests
            .iter()
            .all(|test| !test.caught.contains(&"d9".to_string())),
        "the uncaught defect stands in no caught list"
    );
    // Through the door, the defect is named on standard output.
    let scratch = Scratch::new("uncaught");
    let table_path = scratch.0.join("table.json");
    std::fs::write(
        &table_path,
        r#"{"format": "greenharvest-catch-table/1", "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 100}],
            "catches": [{"defect": "d9", "caught_by": []}]}"#,
    )
    .expect("the table is written");
    let output = Command::new(binary())
        .arg("ingest")
        .arg(&table_path)
        .arg("--out")
        .arg(scratch.0.join("records.json"))
        .output()
        .expect("the binary runs");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("the output is text"),
        "uncaught defect 'd9'\n"
    );
}

#[test]
fn test_a_catcher_naming_no_standing_test_is_refused() {
    let error = read_catch_table_text(
        r#"{"format": "greenharvest-catch-table/1", "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 100}],
            "catches": [{"defect": "d1", "caught_by": ["ghost"]}]}"#,
    )
    .expect_err("the unknown catcher refuses");
    assert_eq!(
        error.to_string(),
        "the defect d1 is recorded as caught by the test ghost, which the catch table does not carry"
    );
}

#[test]
fn test_a_defect_standing_twice_in_the_table_is_refused() {
    let error = read_catch_table_text(
        r#"{"format": "greenharvest-catch-table/1", "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 100}],
            "catches": [{"defect": "d1", "caught_by": ["alpha"]},
                        {"defect": "d1", "caught_by": []}]}"#,
    )
    .expect_err("the repeated defect refuses");
    assert_eq!(
        error.to_string(),
        "the catch table lists the defect d1 twice"
    );
}

#[test]
fn test_a_test_standing_twice_in_the_table_is_refused() {
    let error = read_catch_table_text(
        r#"{"format": "greenharvest-catch-table/1", "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 100},
                      {"name": "alpha", "runtime_ms": 90}],
            "catches": []}"#,
    )
    .expect_err("the repeated test refuses");
    assert_eq!(
        error.to_string(),
        "the catch table lists the test name alpha twice"
    );
}

#[test]
fn test_a_catcher_repeated_in_one_row_is_refused() {
    let error = read_catch_table_text(
        r#"{"format": "greenharvest-catch-table/1", "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 100}],
            "catches": [{"defect": "d1", "caught_by": ["alpha", "alpha"]}]}"#,
    )
    .expect_err("the repeated catcher refuses");
    assert_eq!(
        error.to_string(),
        "the defect d1 names the test alpha more than once as a catcher"
    );
}

#[test]
fn test_the_ingested_records_plan_like_any_other() {
    let table = read_catch_table_text(TABLE).expect("the table reads");
    let direct = read_records_text(
        r#"{"format": "suitediet-records/1", "suite": "probe", "tests": [
            {"name": "alpha", "runtime_ms": 100, "caught": ["d1", "d2"]},
            {"name": "beta", "runtime_ms": 50, "caught": ["d2"]},
            {"name": "gamma", "runtime_ms": 10, "caught": []}
        ]}"#,
    )
    .expect("the record set reads");
    assert_eq!(table.record_set, direct);
    let budget = BigInt::from_i64(120);
    let from_table = build_plan(&table.record_set, &budget).expect("the plan builds");
    let from_direct = build_plan(&direct, &budget).expect("the plan builds");
    assert_eq!(
        canonical_text(&report_document(&from_table)),
        canonical_text(&report_document(&from_direct))
    );
}

#[test]
fn test_a_table_with_a_wrong_format_name_is_refused() {
    let error = read_catch_table_text(
        r#"{"format": "suitediet-records/1", "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 100}],
            "catches": []}"#,
    )
    .expect_err("the wrong format name refuses");
    assert!(
        error
            .to_string()
            .contains("rather than 'greenharvest-catch-table/1'"),
        "the refusal names the standing format, {error}"
    );
}

#[test]
fn test_an_unknown_field_in_the_table_is_refused() {
    let error = read_catch_table_text(
        r#"{"format": "greenharvest-catch-table/1", "suite": "probe", "extra": 1,
            "tests": [{"name": "alpha", "runtime_ms": 100}],
            "catches": []}"#,
    )
    .expect_err("the unknown field refuses");
    assert!(
        error.to_string().contains("carries fields"),
        "the refusal is the exactness wording, {error}"
    );
    let in_entry = read_catch_table_text(
        r#"{"format": "greenharvest-catch-table/1", "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 100, "caught": []}],
            "catches": []}"#,
    )
    .expect_err("the unknown entry field refuses");
    assert!(in_entry.to_string().contains("carries fields"));
}
