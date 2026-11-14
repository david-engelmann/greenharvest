//! The catch table reading, the front end of slice 3. A catch
//! table is the defect keyed reading of the evidence the
//! records hold test keyed, one row per defect naming the
//! tests that caught it beside the tests that ran with the
//! runtime each one measured, under the format name
//! greenharvest-catch-table/1. The reading validates the
//! table exactly as the records reading validates its own
//! documents and transposes it into the held form of a
//! record set, naming the defects no test caught, which
//! stand in no caught list and so cannot stand in the
//! record set at all.

use std::path::Path;

use crate::bigint::BigInt;
use crate::errors::GreenharvestError;
use crate::ingest::{read_file_text, RecordSet, TestRecord};
use crate::jsonvalue::{parse_json, JsonValue, ParseFailure};
use crate::render::{repr_string, repr_value};

pub const CATCH_TABLE_FORMAT: &str = "greenharvest-catch-table/1";

const TABLE_FIELDS: [&str; 4] = ["catches", "format", "suite", "tests"];
const TABLE_TEST_FIELDS: [&str; 2] = ["name", "runtime_ms"];
const CATCH_FIELDS: [&str; 2] = ["caught_by", "defect"];

/// A catch table read and transposed, the record set it
/// makes and the defect identifiers no test caught, sorted.
#[derive(Debug)]
pub struct CatchTable {
    pub record_set: RecordSet,
    pub uncaught: Vec<String>,
}

/// The standings of one catch table read whole, before the
/// transpose. The tests stand in the order the table lists
/// them and the catches stand by defect in the order the
/// table lists them, each row holding its catchers in the
/// order the table names them. The union of slice 4 merges
/// standings of this shape and the transpose reads one.
struct TableStanding {
    suite: String,
    tests: Vec<(String, BigInt)>,
    catches: Vec<(String, Vec<String>)>,
}

fn read_json(text: &str, what: &str) -> Result<JsonValue, GreenharvestError> {
    parse_json(text).map_err(|failure| match failure {
        ParseFailure::Invalid(message) => {
            GreenharvestError::new(format!("the {what} is not valid JSON: {message}"))
        }
        ParseFailure::Boundary => GreenharvestError::new(format!("the {what} is not valid JSON")),
        ParseFailure::DuplicateKey(key) => GreenharvestError::new(format!(
            "a JSON object carries the key {} twice",
            repr_string(&key)
        )),
    })
}

/// The list of a document's field names as the Python refusal
/// spells it, the sorted names in a list representation.
fn fields_spelling(keys: &[&str]) -> String {
    let mut sorted: Vec<&str> = keys.to_vec();
    sorted.sort();
    let items: Vec<JsonValue> = sorted
        .iter()
        .map(|key| JsonValue::Text((*key).to_string()))
        .collect();
    repr_value(&JsonValue::List(items))
}

fn expected_fields_spelling(fields: &[&str]) -> String {
    let items: Vec<JsonValue> = fields
        .iter()
        .map(|key| JsonValue::Text((*key).to_string()))
        .collect();
    repr_value(&JsonValue::List(items))
}

fn check_fields(
    fields: &[(String, JsonValue)],
    expected: &[&str],
    wording: &str,
) -> Result<(), GreenharvestError> {
    let keys: Vec<&str> = fields.iter().map(|(key, _)| key.as_str()).collect();
    let mut sorted_keys = keys.clone();
    sorted_keys.sort();
    if sorted_keys.as_slice() != expected {
        return Err(GreenharvestError::new(format!(
            "{wording} carries fields {} rather than {}",
            fields_spelling(&keys),
            expected_fields_spelling(expected),
        )));
    }
    Ok(())
}

/// Read a catch table document from its text and transpose
/// it into the record set it makes.
fn read_standing(text: &str) -> Result<TableStanding, GreenharvestError> {
    let what = "catch table";
    let document = read_json(text, what)?;
    let fields = match document.as_object() {
        Some(fields) => fields,
        None => {
            return Err(GreenharvestError::new(format!(
                "the {what} is not an object"
            )));
        }
    };
    check_fields(fields, &TABLE_FIELDS, &format!("the {what}"))?;
    let format_value = document.get("format").expect("the field stands");
    let format_matches = matches!(
        format_value,
        JsonValue::Text(text) if text == CATCH_TABLE_FORMAT
    );
    if !format_matches {
        return Err(GreenharvestError::new(format!(
            "the {what} carries the format name {} rather than {}",
            repr_value(format_value),
            repr_string(CATCH_TABLE_FORMAT),
        )));
    }
    let suite = match document.get("suite") {
        Some(JsonValue::Text(text)) if !text.is_empty() => text.clone(),
        _ => {
            return Err(GreenharvestError::new(format!(
                "the {what} carries an empty or non text suite name"
            )));
        }
    };
    let test_entries = match document.get("tests") {
        Some(JsonValue::List(entries)) => entries,
        _ => {
            return Err(GreenharvestError::new(format!(
                "the {what} tests field is not a list"
            )));
        }
    };
    if test_entries.is_empty() {
        return Err(GreenharvestError::new(format!(
            "the {what} tests list is empty"
        )));
    }
    let mut names: Vec<String> = Vec::new();
    let mut runtimes: Vec<BigInt> = Vec::new();
    for entry in test_entries {
        let entry_fields = match entry.as_object() {
            Some(fields) => fields,
            None => {
                return Err(GreenharvestError::new(format!(
                    "a test entry in the {what} is not an object"
                )));
            }
        };
        check_fields(
            entry_fields,
            &TABLE_TEST_FIELDS,
            "a test entry in the catch table",
        )?;
        let name = match entry.get("name") {
            Some(JsonValue::Text(text)) if !text.is_empty() => text.clone(),
            _ => {
                return Err(GreenharvestError::new(format!(
                    "a test entry in the {what} carries an empty or non text name"
                )));
            }
        };
        if names.iter().any(|standing| standing == &name) {
            return Err(GreenharvestError::new(format!(
                "the {what} lists the test name {name} twice"
            )));
        }
        let runtime = match entry.get("runtime_ms") {
            Some(JsonValue::Int(value)) => BigInt::from_i64(*value),
            Some(JsonValue::BigInt(text)) => {
                BigInt::from_decimal(text).expect("a stored integer token reads back")
            }
            _ => {
                return Err(GreenharvestError::new(format!(
                    "the test {name} carries a runtime that is not a positive whole number of milliseconds"
                )));
            }
        };
        if !runtime.is_positive() {
            return Err(GreenharvestError::new(format!(
                "the test {name} carries a runtime that is not a positive whole number of milliseconds"
            )));
        }
        names.push(name);
        runtimes.push(runtime);
    }
    let catch_entries = match document.get("catches") {
        Some(JsonValue::List(entries)) => entries,
        _ => {
            return Err(GreenharvestError::new(format!(
                "the {what} catches field is not a list"
            )));
        }
    };
    let mut seen_defects: Vec<String> = Vec::new();
    let mut catches: Vec<(String, Vec<String>)> = Vec::new();
    for entry in catch_entries {
        let entry_fields = match entry.as_object() {
            Some(fields) => fields,
            None => {
                return Err(GreenharvestError::new(format!(
                    "a catch entry in the {what} is not an object"
                )));
            }
        };
        check_fields(
            entry_fields,
            &CATCH_FIELDS,
            "a catch entry in the catch table",
        )?;
        let defect = match entry.get("defect") {
            Some(JsonValue::Text(text)) if !text.is_empty() => text.clone(),
            _ => {
                return Err(GreenharvestError::new(format!(
                    "a catch entry in the {what} carries an empty or non text defect identifier"
                )));
            }
        };
        if seen_defects.iter().any(|standing| standing == &defect) {
            return Err(GreenharvestError::new(format!(
                "the {what} lists the defect {defect} twice"
            )));
        }
        seen_defects.push(defect.clone());
        let catchers = match entry.get("caught_by") {
            Some(JsonValue::List(items)) => items,
            _ => {
                return Err(GreenharvestError::new(format!(
                    "the defect {defect} carries a caught_by field that is not a list"
                )));
            }
        };
        let mut row: Vec<String> = Vec::new();
        for catcher in catchers {
            let catcher_name = match catcher {
                JsonValue::Text(text) if !text.is_empty() => text.clone(),
                _ => {
                    return Err(GreenharvestError::new(format!(
                        "the defect {defect} names an empty or non text test as a catcher"
                    )));
                }
            };
            if row.iter().any(|standing| standing == &catcher_name) {
                return Err(GreenharvestError::new(format!(
                    "the defect {defect} names the test {catcher_name} more than once as a catcher"
                )));
            }
            row.push(catcher_name);
        }
        catches.push((defect, row));
    }
    let tests: Vec<(String, BigInt)> = names.into_iter().zip(runtimes).collect();
    Ok(TableStanding {
        suite,
        tests,
        catches,
    })
}

/// Transpose one standing into the record set it makes,
/// answering the catcher membership question against the
/// tests the standing carries, which for the union is the
/// tests of every table standing together.
fn transpose(standing: TableStanding) -> Result<CatchTable, GreenharvestError> {
    let names: Vec<&str> = standing
        .tests
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    let mut caught_by_test: Vec<Vec<String>> = names.iter().map(|_| Vec::new()).collect();
    let mut uncaught: Vec<String> = Vec::new();
    for (defect, row) in &standing.catches {
        for catcher_name in row {
            let index = match names.iter().position(|standing| standing == catcher_name) {
                Some(index) => index,
                None => {
                    return Err(GreenharvestError::new(format!(
                        "the defect {defect} is recorded as caught by the test {catcher_name}, which the catch table does not carry"
                    )));
                }
            };
            caught_by_test[index].push(defect.clone());
        }
        if row.is_empty() {
            uncaught.push(defect.clone());
        }
    }
    let mut tests: Vec<TestRecord> = Vec::new();
    for (index, (name, runtime)) in standing.tests.iter().enumerate() {
        let mut caught = caught_by_test[index].clone();
        caught.sort();
        tests.push(TestRecord {
            name: name.clone(),
            runtime_ms: runtime.clone(),
            caught,
        });
    }
    tests.sort_by(|a, b| a.name.cmp(&b.name));
    uncaught.sort();
    Ok(CatchTable {
        record_set: RecordSet {
            suite: standing.suite,
            tests,
        },
        uncaught,
    })
}

/// Read a catch table document from its text and transpose
/// it, as the reading stood before slice 4.
pub fn read_catch_table_text(text: &str) -> Result<CatchTable, GreenharvestError> {
    transpose(read_standing(text)?)
}

/// Merge the standings of two or more tables into the one
/// standing of their union, refusing a suite disagreement
/// and a test standing at two runtimes.
fn union_standings(standings: Vec<TableStanding>) -> Result<TableStanding, GreenharvestError> {
    let mut iter = standings.into_iter();
    let first = iter.next().expect("the union reads at least one table");
    let suite = first.suite.clone();
    let mut tests: Vec<(String, BigInt)> = Vec::new();
    let mut catches: Vec<(String, Vec<String>)> = Vec::new();
    for standing in std::iter::once(first).chain(iter) {
        if standing.suite != suite {
            return Err(GreenharvestError::new(format!(
                "the catch tables do not agree on the suite, {} against {}",
                repr_string(&suite),
                repr_string(&standing.suite),
            )));
        }
        for (name, runtime) in standing.tests {
            match tests.iter().position(|(held, _)| held == &name) {
                Some(index) => {
                    if tests[index].1.cmp(&runtime) != std::cmp::Ordering::Equal {
                        return Err(GreenharvestError::new(format!(
                            "the test {name} stands at two runtimes across the catch tables, {} and {}",
                            tests[index].1.to_decimal_string(),
                            runtime.to_decimal_string(),
                        )));
                    }
                }
                None => tests.push((name, runtime)),
            }
        }
        for (defect, row) in standing.catches {
            match catches.iter().position(|(held, _)| held == &defect) {
                Some(index) => {
                    for catcher in row {
                        if !catches[index].1.contains(&catcher) {
                            catches[index].1.push(catcher);
                        }
                    }
                }
                None => catches.push((defect, row)),
            }
        }
    }
    Ok(TableStanding {
        suite,
        tests,
        catches,
    })
}

/// Read two or more catch table documents from their texts
/// and transpose their union, the slice 4 reading.
pub fn read_catch_tables_text(texts: &[&str]) -> Result<CatchTable, GreenharvestError> {
    let mut standings = Vec::new();
    for text in texts {
        standings.push(read_standing(text)?);
    }
    transpose(union_standings(standings)?)
}

/// Read one or more catch table documents from local files
/// and transpose their union.
pub fn read_catch_tables(paths: &[std::path::PathBuf]) -> Result<CatchTable, GreenharvestError> {
    let mut standings = Vec::new();
    for path in paths {
        let text = read_file_text(path, "catch table")?;
        standings.push(read_standing(&text)?);
    }
    transpose(union_standings(standings)?)
}

/// Read a catch table document from a local file and
/// transpose it.
pub fn read_catch_table(path: &Path) -> Result<CatchTable, GreenharvestError> {
    let text = read_file_text(path, "catch table")?;
    read_catch_table_text(&text)
}
