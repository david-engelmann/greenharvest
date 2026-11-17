//! The pins of slice 11, the catch table precedence, over
//! synthetic tables the suite writes itself. Each pin hands
//! the reading a document or a pair of documents carrying
//! exactly two faults and asserts the refusal that speaks,
//! word for word, so a change that reorders the reading
//! breaks the pin that names the two standings it traded.

use greenharvest::catchtable::{read_catch_table_text, read_catch_tables_text};

fn refusal(text: &str) -> String {
    read_catch_table_text(text)
        .expect_err("the table refuses")
        .message()
        .to_string()
}

fn union_refusal(texts: &[&str]) -> String {
    read_catch_tables_text(texts)
        .expect_err("the tables refuse")
        .message()
        .to_string()
}

#[test]
fn test_a_repeated_key_speaks_ahead_of_a_format_fault() {
    let message = refusal(
        r#"{"format": "suitediet-records/1", "format": "greenharvest-catch-table/1",
            "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 100}],
            "catches": []}"#,
    );
    assert_eq!(message, "a JSON object carries the key 'format' twice");
}

#[test]
fn test_a_fields_fault_speaks_ahead_of_a_format_fault() {
    let message = refusal(
        r#"{"format": "suitediet-records/1", "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 100}]}"#,
    );
    assert_eq!(
        message,
        "the catch table carries fields ['format', 'suite', 'tests'] rather than ['catches', 'format', 'suite', 'tests']"
    );
}

#[test]
fn test_a_format_fault_speaks_ahead_of_a_tests_fault() {
    let message = refusal(
        r#"{"format": "suitediet-records/1", "suite": "probe",
            "tests": [],
            "catches": []}"#,
    );
    assert_eq!(
        message,
        "the catch table carries the format name 'suitediet-records/1' rather than 'greenharvest-catch-table/1'"
    );
}

#[test]
fn test_a_tests_fault_speaks_ahead_of_a_row_fault() {
    let message = refusal(
        r#"{"format": "greenharvest-catch-table/1", "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 100},
                      {"name": "alpha", "runtime_ms": 90}],
            "catches": [{"defect": "d1", "caught_by": ["ghost"]}]}"#,
    );
    assert_eq!(message, "the catch table lists the test name alpha twice");
}

#[test]
fn test_an_earlier_tables_fault_speaks_ahead_of_a_later_tables_fault() {
    let message = union_refusal(&[
        r#"{"format": "greenharvest-catch-table/1", "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 100}],
            "catches": [{"defect": "d1", "caught_by": ["alpha"]},
                        {"defect": "d1", "caught_by": []}]}"#,
        r#"{"format": "suitediet-records/1", "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 100}],
            "catches": []}"#,
    ]);
    assert_eq!(message, "the catch table lists the defect d1 twice");
}

#[test]
fn test_an_earlier_test_entry_fault_speaks_ahead_of_a_later_one() {
    let message = refusal(
        r#"{"format": "greenharvest-catch-table/1", "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 0},
                      {"name": "alpha", "runtime_ms": 90}],
            "catches": []}"#,
    );
    assert_eq!(
        message,
        "the test alpha carries a runtime that is not a positive whole number of milliseconds"
    );
}

#[test]
fn test_an_earlier_rows_fault_speaks_ahead_of_a_later_rows_fault() {
    let message = refusal(
        r#"{"format": "greenharvest-catch-table/1", "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 100}],
            "catches": [{"defect": "d1", "caught_by": ["alpha", "alpha"]},
                        {"defect": "d1", "caught_by": []}]}"#,
    );
    assert_eq!(
        message,
        "the defect d1 names the test alpha more than once as a catcher"
    );
}

#[test]
fn test_the_first_unknown_catcher_in_row_order_speaks() {
    let message = refusal(
        r#"{"format": "greenharvest-catch-table/1", "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 100}],
            "catches": [{"defect": "d1", "caught_by": ["ghost_one"]},
                        {"defect": "d2", "caught_by": ["ghost_two"]}]}"#,
    );
    assert_eq!(
        message,
        "the defect d1 is recorded as caught by the test ghost_one, which the catch table does not carry"
    );
}

#[test]
fn test_the_first_unknown_catcher_in_a_row_speaks() {
    let message = refusal(
        r#"{"format": "greenharvest-catch-table/1", "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 100}],
            "catches": [{"defect": "d1", "caught_by": ["ghost_one", "ghost_two"]}]}"#,
    );
    assert_eq!(
        message,
        "the defect d1 is recorded as caught by the test ghost_one, which the catch table does not carry"
    );
}

#[test]
fn test_an_unknown_catcher_of_an_earlier_table_speaks_ahead_of_a_later_tables() {
    let message = union_refusal(&[
        r#"{"format": "greenharvest-catch-table/1", "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 100}],
            "catches": [{"defect": "d1", "caught_by": ["ghost_one"]}]}"#,
        r#"{"format": "greenharvest-catch-table/1", "suite": "probe",
            "tests": [{"name": "beta", "runtime_ms": 50}],
            "catches": [{"defect": "d2", "caught_by": ["ghost_two"]}]}"#,
    ]);
    assert_eq!(
        message,
        "the defect d1 is recorded as caught by the test ghost_one, which the catch table does not carry"
    );
}

#[test]
fn test_a_doubled_catcher_speaks_ahead_of_a_later_unknown_catcher() {
    let message = refusal(
        r#"{"format": "greenharvest-catch-table/1", "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 100}],
            "catches": [{"defect": "d1", "caught_by": ["alpha", "alpha"]},
                        {"defect": "d2", "caught_by": ["ghost"]}]}"#,
    );
    assert_eq!(
        message,
        "the defect d1 names the test alpha more than once as a catcher"
    );
}

#[test]
fn test_a_doubled_catcher_speaks_ahead_of_an_earlier_unknown_catcher() {
    let message = refusal(
        r#"{"format": "greenharvest-catch-table/1", "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 100}],
            "catches": [{"defect": "d1", "caught_by": ["ghost"]},
                        {"defect": "d2", "caught_by": ["alpha", "alpha"]}]}"#,
    );
    assert_eq!(
        message,
        "the defect d2 names the test alpha more than once as a catcher"
    );
}

#[test]
fn test_a_defect_listed_twice_speaks_ahead_of_an_unknown_catcher() {
    let message = refusal(
        r#"{"format": "greenharvest-catch-table/1", "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 100}],
            "catches": [{"defect": "d1", "caught_by": ["ghost"]},
                        {"defect": "d1", "caught_by": ["alpha"]}]}"#,
    );
    assert_eq!(message, "the catch table lists the defect d1 twice");
}

#[test]
fn test_a_suite_disagreement_speaks_ahead_of_a_runtime_disagreement() {
    let message = union_refusal(&[
        r#"{"format": "greenharvest-catch-table/1", "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 100}],
            "catches": []}"#,
        r#"{"format": "greenharvest-catch-table/1", "suite": "billing",
            "tests": [{"name": "alpha", "runtime_ms": 90}],
            "catches": []}"#,
    ]);
    assert_eq!(
        message,
        "the catch tables do not agree on the suite, 'probe' against 'billing'"
    );
}

#[test]
fn test_a_runtime_disagreement_speaks_ahead_of_an_unknown_catcher() {
    let message = union_refusal(&[
        r#"{"format": "greenharvest-catch-table/1", "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 100}],
            "catches": [{"defect": "d1", "caught_by": ["ghost"]}]}"#,
        r#"{"format": "greenharvest-catch-table/1", "suite": "probe",
            "tests": [{"name": "alpha", "runtime_ms": 90}],
            "catches": []}"#,
    ]);
    assert_eq!(
        message,
        "the test alpha stands at two runtimes across the catch tables, 100 and 90"
    );
}
