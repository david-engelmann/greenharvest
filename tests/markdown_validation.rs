//! The pins of slice 10, the markdown validation, over
//! synthetic fixtures the suite writes itself and over the
//! binary's validate-markdown door.

use std::path::PathBuf;
use std::process::Command;

use greenharvest::bigint::BigInt;
use greenharvest::ingest::{read_records_text, RecordSet};
use greenharvest::jsonvalue::JsonValue;
use greenharvest::report::{markdown_text, read_report_text, report_document, validate_markdown};
use greenharvest::{build_plan, canonical_text};

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_greenharvest"))
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let path = std::env::temp_dir().join(format!(
            "greenharvest-markdown-validation-{}-{tag}",
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
/// delta, which catches nothing. Its markdown states the
/// ranking of the four tests, the three removal packs with
/// their overlap, and the totals of two defects before the
/// plan and two after.
const RECORDS: &str = r#"{
  "format": "suitediet-records/1",
  "suite": "markdown validation suite",
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

fn genuine_markdown() -> String {
    markdown_text(&genuine_report())
}

const DISAGREEMENT: &str = "the markdown disagrees with the report it was rendered from";

/// The error wording a tampered markdown refuses with. The
/// mutation must move the markdown, so a pin whose tamper
/// silently changed nothing fails here rather than passing
/// on a genuine document.
fn disagreement(mutate: impl FnOnce(&mut String)) -> String {
    let mut markdown = genuine_markdown();
    mutate(&mut markdown);
    assert_ne!(
        markdown,
        genuine_markdown(),
        "the tamper moves the markdown"
    );
    validate_markdown(&genuine_report(), &markdown)
        .expect_err("the tampered markdown refuses")
        .to_string()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn test_a_genuine_markdown_validates() {
    validate_markdown(&genuine_report(), &genuine_markdown())
        .expect("the genuine markdown validates");
    // The report as read back from its own text under the
    // standing report reading validates its rendering whole.
    let text = canonical_text(&genuine_report());
    let read = read_report_text(&text).expect("the report reads back");
    validate_markdown(&read, &genuine_markdown()).expect("the read report validates");
    let scratch = Scratch::new("genuine");
    let report_path = scratch.write("report.json", &text);
    let markdown_path = scratch.write("report.md", &genuine_markdown());
    let output = Command::new(binary())
        .arg("validate-markdown")
        .arg(&report_path)
        .arg(&markdown_path)
        .output()
        .expect("the door runs");
    assert!(
        output.status.success(),
        "the door exits 0 on a genuine markdown, stderr: {}",
        stderr(&output)
    );
    assert!(output.stdout.is_empty(), "the door prints nothing");
}

#[test]
fn test_a_markdown_with_an_altered_total_is_refused() {
    let message = disagreement(|markdown| {
        *markdown = markdown.replace(
            "catches 2 defects before the plan",
            "catches 9 defects before the plan",
        );
    });
    assert_eq!(message, DISAGREEMENT);
    // Through the door the same drift refuses at exit code 2
    // in the disagreement words.
    let altered = genuine_markdown().replace(
        "catches 2 defects before the plan",
        "catches 9 defects before the plan",
    );
    let scratch = Scratch::new("total");
    let report_path = scratch.write("report.json", &canonical_text(&genuine_report()));
    let markdown_path = scratch.write("report.md", &altered);
    let output = Command::new(binary())
        .arg("validate-markdown")
        .arg(&report_path)
        .arg(&markdown_path)
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
fn test_a_markdown_with_a_flipped_disposition_is_refused() {
    let message = disagreement(|markdown| {
        *markdown = markdown.replace(
            "beta is proposed for quarantine",
            "beta is held in the suite",
        );
    });
    assert_eq!(message, DISAGREEMENT);
}

#[test]
fn test_a_markdown_with_two_ranking_rows_swapped_is_refused() {
    let message = disagreement(|markdown| {
        let mut lines: Vec<&str> = markdown.split('\n').collect();
        let first = lines
            .iter()
            .position(|line| line.starts_with("| 1 |"))
            .expect("the first ranking row stands");
        let second = lines
            .iter()
            .position(|line| line.starts_with("| 2 |"))
            .expect("the second ranking row stands");
        lines.swap(first, second);
        *markdown = lines.join("\n");
    });
    assert_eq!(message, DISAGREEMENT);
}

#[test]
fn test_a_markdown_with_a_dropped_overlap_sentence_is_refused() {
    let message = disagreement(|markdown| {
        *markdown = markdown.replace(
            " Overlap with the tests still standing after the plan. d2 is also caught by gamma.",
            "",
        );
    });
    assert_eq!(message, DISAGREEMENT);
}

#[test]
fn test_a_markdown_with_an_altered_value_is_refused() {
    let message = disagreement(|markdown| {
        *markdown = markdown.replace("0.050000", "0.060000");
    });
    assert_eq!(message, DISAGREEMENT);
}

#[test]
fn test_a_markdown_from_another_budget_is_refused() {
    let wider = build_plan(&record_set(), &BigInt::from_i64(160)).expect("the wider plan builds");
    let wider_markdown = markdown_text(&report_document(&wider));
    assert_ne!(wider_markdown, genuine_markdown());
    let error = validate_markdown(&genuine_report(), &wider_markdown)
        .expect_err("the markdown of another budget refuses");
    assert_eq!(error.to_string(), DISAGREEMENT);
}

#[test]
fn test_a_markdown_missing_its_final_newline_is_refused() {
    let message = disagreement(|markdown| {
        let stripped = markdown
            .strip_suffix('\n')
            .expect("the rendering ends with a newline")
            .to_string();
        *markdown = stripped;
    });
    assert_eq!(message, DISAGREEMENT);
}

#[test]
fn test_a_markdown_with_translated_line_endings_validates() {
    // The standing file reading translates the line endings
    // of every document it reads, so a markdown written with
    // carriage returns standing before its newlines reads as
    // the rendering itself and validates through the door.
    let scratch = Scratch::new("endings");
    let report_path = scratch.write("report.json", &canonical_text(&genuine_report()));
    let markdown_path = scratch.write("report.md", &genuine_markdown().replace('\n', "\r\n"));
    let output = Command::new(binary())
        .arg("validate-markdown")
        .arg(&report_path)
        .arg(&markdown_path)
        .output()
        .expect("the door runs");
    assert!(
        output.status.success(),
        "the door exits 0 on a markdown with translated line endings, stderr: {}",
        stderr(&output)
    );
    assert!(output.stdout.is_empty(), "the door prints nothing");
}

#[test]
fn test_the_markdown_of_a_plan_with_no_removals_validates() {
    let plan = build_plan(&record_set(), &BigInt::from_i64(100000)).expect("the plan builds");
    let report = report_document(&plan);
    let markdown = markdown_text(&report);
    assert!(
        markdown.contains("No test stands below the cut line, so no removal is proposed."),
        "the rendering states that no removal is proposed: {markdown}"
    );
    validate_markdown(&report, &markdown).expect("the markdown of an empty plan validates");
}
