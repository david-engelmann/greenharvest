//! Recorded conformance for Rust port slice 1.
//!
//! Every case in tests/conformance/port1_cases.json was recorded
//! from the landed Python, and the boundary cases carry the
//! port's stated answer where the Python crashes or disagrees,
//! each marked in the recording. The recordings are generated and
//! never hand edited. The census of every section is asserted
//! here as well as carried in the recording, so a regenerated
//! recording that quietly drops a case fails the suite.

use std::collections::BTreeMap;
use std::path::PathBuf;

use greenharvest::bigint::BigInt;
use greenharvest::ingest::{
    quarantine_document, read_quarantine_text, read_records_text, read_remaining_text,
    records_document,
};
use greenharvest::jsonvalue::{parse_json, JsonValue, ParseFailure};
use greenharvest::plan::build_plan;
use greenharvest::quarantine::{build_quarantine, remaining_records, restore_records};
use greenharvest::rational::{render_value, Rational};
use greenharvest::render::repr_value;
use greenharvest::report::{markdown_text, report_document};
use greenharvest::{canonical_text, parse_budget};

const TOTAL_CASES: usize = 173;
const BOUNDARY_CASES: usize = 21;

fn expected_sections() -> Vec<(&'static str, usize)> {
    vec![
        ("reader", 66),
        ("plan", 14),
        ("restore", 6),
        ("render", 14),
        ("repr", 12),
        ("budget", 42),
        ("door", 19),
    ]
}

fn load() -> JsonValue {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/conformance/port1_cases.json");
    let text = std::fs::read_to_string(&path).expect("the recordings stand on disk");
    parse_json(&text).expect("the recordings parse")
}

fn cases() -> Vec<JsonValue> {
    let document = load();
    document
        .get("cases")
        .and_then(JsonValue::as_list)
        .expect("the recordings carry their cases")
        .to_vec()
}

fn field<'a>(value: &'a JsonValue, name: &str) -> &'a JsonValue {
    value
        .get(name)
        .unwrap_or_else(|| panic!("a case carries the field {name}"))
}

fn field_text<'a>(value: &'a JsonValue, name: &str) -> &'a str {
    field(value, name)
        .as_text()
        .unwrap_or_else(|| panic!("the field {name} stands as text"))
}

fn case_label(case: &JsonValue) -> String {
    format!(
        "{} / {}",
        field_text(case, "section"),
        field_text(case, "name")
    )
}

fn expect_error(case: &JsonValue) -> Option<&str> {
    let expect = field(case, "expect");
    expect.get("error").and_then(|error| {
        error
            .as_list()
            .and_then(|parts| parts.get(1))
            .and_then(JsonValue::as_text)
    })
}

fn expect_ok(case: &JsonValue) -> Option<&JsonValue> {
    field(case, "expect").get("ok")
}

#[test]
fn census_stands_as_recorded() {
    let document = load();
    let recorded_total = match document.get("case_count") {
        Some(JsonValue::Int(value)) => *value as usize,
        _ => panic!("the recordings carry their case count"),
    };
    assert_eq!(recorded_total, TOTAL_CASES, "the recorded total stands");
    let recorded_boundary = match document.get("boundary_count") {
        Some(JsonValue::Int(value)) => *value as usize,
        _ => panic!("the recordings carry their boundary count"),
    };
    assert_eq!(recorded_boundary, BOUNDARY_CASES);
    let all = cases();
    assert_eq!(all.len(), TOTAL_CASES, "the case list matches its count");
    let mut computed: BTreeMap<String, usize> = BTreeMap::new();
    let mut boundary = 0;
    for case in &all {
        *computed
            .entry(field_text(case, "section").to_string())
            .or_insert(0) += 1;
        if case.get("boundary").is_some() {
            boundary += 1;
        }
    }
    assert_eq!(boundary, BOUNDARY_CASES, "the marked boundary cases stand");
    for (section, count) in expected_sections() {
        assert_eq!(
            computed.get(section),
            Some(&count),
            "the {section} census stands as asserted"
        );
        let recorded = document
            .get("sections")
            .and_then(|sections| sections.get(section));
        assert!(
            matches!(recorded, Some(JsonValue::Int(value)) if *value == count as i64),
            "the recorded {section} census stands as asserted"
        );
    }
    assert_eq!(computed.len(), expected_sections().len());
}

fn run_reader_case(case: &JsonValue) {
    let inputs = field(case, "inputs");
    let kind = field_text(inputs, "kind");
    let text = field_text(inputs, "text");
    let outcome = match kind {
        "records" => read_records_text(text).map(|set| canonical_text(&records_document(&set))),
        "remaining" => read_remaining_text(text).map(|set| canonical_text(&records_document(&set))),
        _ => read_quarantine_text(text)
            .map(|quarantine| canonical_text(&quarantine_document(&quarantine))),
    };
    match (outcome, expect_ok(case), expect_error(case)) {
        (Ok(held), Some(expected), _) => {
            assert_eq!(
                held,
                expected.as_text().expect("the held text stands"),
                "{}",
                case_label(case)
            );
        }
        (Err(error), _, Some(message)) => {
            assert_eq!(error.message(), message, "{}", case_label(case));
        }
        (outcome, _, _) => {
            panic!(
                "{}: outcome and expectation disagree: {outcome:?}",
                case_label(case)
            );
        }
    }
}

#[test]
fn reader_cases() {
    for case in cases()
        .iter()
        .filter(|case| field_text(case, "kind") == "reader")
    {
        run_reader_case(case);
    }
}

#[test]
fn plan_cases() {
    for case in cases()
        .iter()
        .filter(|case| field_text(case, "kind") == "plan")
    {
        let inputs = field(case, "inputs");
        let text = field_text(inputs, "text");
        let budget = BigInt::from_decimal(field_text(inputs, "budget"))
            .expect("the recorded budget stands as digits");
        let outcome = read_records_text(text).and_then(|record_set| {
            build_plan(&record_set, &budget).map(|plan| {
                let report = report_document(&plan);
                (
                    canonical_text(&report),
                    markdown_text(&report),
                    canonical_text(&quarantine_document(&build_quarantine(&record_set, &plan))),
                    canonical_text(&records_document(&remaining_records(&record_set, &plan))),
                )
            })
        });
        match (outcome, expect_ok(case), expect_error(case)) {
            (Ok((report, markdown, quarantine, remaining)), Some(expected), _) => {
                for (name, actual) in [
                    ("report", &report),
                    ("markdown", &markdown),
                    ("quarantine", &quarantine),
                    ("remaining", &remaining),
                ] {
                    let recorded = expected
                        .get(name)
                        .and_then(JsonValue::as_text)
                        .unwrap_or_else(|| panic!("the plan expectation carries {name}"));
                    assert_eq!(actual, recorded, "{}: {name}", case_label(case));
                }
            }
            (Err(error), _, Some(message)) => {
                assert_eq!(error.message(), message, "{}", case_label(case));
            }
            (outcome, _, _) => panic!(
                "{}: outcome and expectation disagree: {outcome:?}",
                case_label(case)
            ),
        }
    }
}

#[test]
fn restore_cases() {
    for case in cases()
        .iter()
        .filter(|case| field_text(case, "kind") == "restore")
    {
        let inputs = field(case, "inputs");
        let outcome = read_remaining_text(field_text(inputs, "remaining")).and_then(|remaining| {
            read_quarantine_text(field_text(inputs, "quarantine")).and_then(|quarantine| {
                restore_records(&remaining, &quarantine)
                    .map(|set| canonical_text(&records_document(&set)))
            })
        });
        match (outcome, expect_ok(case), expect_error(case)) {
            (Ok(restored), Some(expected), _) => {
                assert_eq!(
                    restored,
                    expected.as_text().expect("the restored text stands"),
                    "{}",
                    case_label(case)
                );
            }
            (Err(error), _, Some(message)) => {
                assert_eq!(error.message(), message, "{}", case_label(case));
            }
            (outcome, _, _) => panic!(
                "{}: outcome and expectation disagree: {outcome:?}",
                case_label(case)
            ),
        }
    }
}

#[test]
fn render_cases() {
    for case in cases()
        .iter()
        .filter(|case| field_text(case, "kind") == "render")
    {
        let inputs = field(case, "inputs");
        let numerator = BigInt::from_decimal(field_text(inputs, "num"))
            .expect("the numerator stands as digits");
        let denominator = BigInt::from_decimal(field_text(inputs, "den"))
            .expect("the denominator stands as digits");
        let rendered = render_value(&Rational::new(numerator, denominator));
        let expected = expect_ok(case)
            .and_then(JsonValue::as_text)
            .expect("the render expectation stands");
        assert_eq!(rendered, expected, "{}", case_label(case));
    }
}

#[test]
fn repr_cases() {
    for case in cases()
        .iter()
        .filter(|case| field_text(case, "kind") == "repr")
    {
        let inputs = field(case, "inputs");
        let expect = field(case, "expect");
        match parse_json(field_text(inputs, "text")) {
            Ok(value) => {
                let expected = expect
                    .get("ok")
                    .and_then(JsonValue::as_text)
                    .expect("the representation expectation stands");
                assert_eq!(repr_value(&value), expected, "{}", case_label(case));
            }
            Err(ParseFailure::Invalid(message)) => {
                let expected = expect
                    .get("invalid")
                    .and_then(JsonValue::as_text)
                    .expect("the invalid expectation stands");
                assert_eq!(message, expected, "{}", case_label(case));
            }
            Err(ParseFailure::DuplicateKey(key)) => {
                let expected = expect
                    .get("duplicate")
                    .and_then(JsonValue::as_text)
                    .expect("the duplicate expectation stands");
                assert_eq!(key, expected, "{}", case_label(case));
            }
            Err(ParseFailure::Boundary) => {
                assert!(
                    expect.get("boundary").is_some(),
                    "{}: the boundary stands marked",
                    case_label(case)
                );
            }
        }
    }
}

#[test]
fn budget_cases() {
    for case in cases()
        .iter()
        .filter(|case| field_text(case, "kind") == "budget")
    {
        let inputs = field(case, "inputs");
        match (
            parse_budget(field_text(inputs, "raw")),
            expect_ok(case),
            expect_error(case),
        ) {
            (Ok(value), Some(expected), _) => {
                assert_eq!(
                    value.to_decimal_string(),
                    expected.as_text().expect("the value stands"),
                    "{}",
                    case_label(case)
                );
            }
            (Err(error), _, Some(message)) => {
                assert_eq!(error.message(), message, "{}", case_label(case));
            }
            (outcome, _, _) => panic!(
                "{}: outcome and expectation disagree: {outcome:?}",
                case_label(case)
            ),
        }
    }
}

/// The door standings the closing slice amends, with the
/// answer that slice states standing in place of the recorded
/// one. The amendment paragraph of the closing slice retires
/// the argument surface sentences of slice 1, under which a
/// malformed invocation showed the two usage lines of the old
/// shell and an option like budget value was read as the
/// budget argument itself. The closing slice carries the
/// Python parser's model whole and writes one short usage line
/// per parser, so the three usage cases answer with the line
/// of the parser that refuses and the option like value case
/// answers with the usage refusal the parser gives it. The
/// recordings stand unchanged as data, and every other
/// standing of these cases is asserted as recorded.
fn amended_door_stderr(name: &str) -> Option<&'static str> {
    match name {
        "no arguments shows the usage lines" => Some("usage: greenharvest {plan,restore} ...\n"),
        "an unknown command shows the usage lines" => {
            Some("usage: greenharvest {plan,restore} ...\n")
        }
        "plan without its flags shows the usage lines" => {
            Some("usage: greenharvest plan RECORDS --budget-ms BUDGET --out-dir OUT_DIR\n")
        }
        "plan refuses an option like budget value" => {
            Some("usage: greenharvest plan RECORDS --budget-ms BUDGET --out-dir OUT_DIR\n")
        }
        _ => None,
    }
}

#[test]
fn door_cases() {
    let binary = env!("CARGO_BIN_EXE_greenharvest");
    for (index, case) in cases()
        .iter()
        .filter(|case| field_text(case, "kind") == "door")
        .enumerate()
    {
        let label = case_label(case);
        let inputs = field(case, "inputs");
        let expect = field(case, "expect");
        let dir = std::env::temp_dir().join(format!(
            "greenharvest-conformance-{}-{index}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the scratch directory stands");
        let dir_text = dir.to_string_lossy().into_owned();
        let substitute = |text: &str| text.replace("{dir}", &dir_text);
        if let Some(files) = inputs.get("files").and_then(JsonValue::as_object) {
            for (rel, content) in files {
                let path = dir.join(rel);
                std::fs::create_dir_all(path.parent().expect("a parent stands"))
                    .expect("the parent stands");
                match content {
                    JsonValue::Null => {
                        std::fs::create_dir_all(&path).expect("the directory stands");
                    }
                    JsonValue::Object(_) => {
                        let bytes: Vec<u8> = content
                            .get("bytes")
                            .and_then(JsonValue::as_list)
                            .expect("the byte list stands")
                            .iter()
                            .map(|byte| match byte {
                                JsonValue::Int(value) => *value as u8,
                                _ => panic!("a byte stands as an integer"),
                            })
                            .collect();
                        std::fs::write(&path, bytes).expect("the file stands");
                    }
                    JsonValue::Text(text) => {
                        std::fs::write(&path, substitute(text)).expect("the file stands");
                    }
                    _ => panic!("a file content stands"),
                }
            }
        }
        let argv: Vec<String> = inputs
            .get("argv")
            .and_then(JsonValue::as_list)
            .expect("the argv stands")
            .iter()
            .map(|part| substitute(part.as_text().expect("an argument stands as text")))
            .collect();
        let output = std::process::Command::new(binary)
            .args(&argv)
            .output()
            .expect("the binary runs");
        let status = output.status.code().expect("the exit code stands");
        let expected_status = match expect.get("status") {
            Some(JsonValue::Int(value)) => *value as i32,
            _ => panic!("the status expectation stands"),
        };
        assert_eq!(status, expected_status, "{label}: status");
        let stderr = String::from_utf8(output.stderr).expect("the stderr is text");
        let stderr = stderr.replace(&dir_text, "{dir}");
        if let Some(expected) = expect.get("stderr").and_then(JsonValue::as_text) {
            let expected = amended_door_stderr(field_text(case, "name")).unwrap_or(expected);
            assert_eq!(stderr, expected, "{label}: stderr");
        }
        if let Some(prefix) = expect.get("stderr_prefix").and_then(JsonValue::as_text) {
            assert!(
                stderr.starts_with(prefix),
                "{label}: stderr {stderr:?} carries the prefix {prefix:?}"
            );
        }
        let stdout = String::from_utf8(output.stdout).expect("the stdout is text");
        let expected_stdout = expect
            .get("stdout")
            .and_then(JsonValue::as_text)
            .expect("the stdout expectation stands");
        assert_eq!(stdout, expected_stdout, "{label}: stdout");
        if let Some(outputs) = expect.get("outputs").and_then(JsonValue::as_object) {
            for (rel, expected) in outputs {
                let actual = std::fs::read_to_string(dir.join(rel))
                    .unwrap_or_else(|_| panic!("{label}: the output {rel} stands"));
                assert_eq!(
                    actual,
                    expected.as_text().expect("the output text stands"),
                    "{label}: output {rel}"
                );
            }
        }
        if let Some(absent) = expect.get("absent").and_then(JsonValue::as_list) {
            for rel in absent {
                let rel = rel.as_text().expect("an absent path stands as text");
                assert!(!dir.join(rel).exists(), "{label}: {rel} stands absent");
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
