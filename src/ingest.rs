//! Reading of the measured record sets suitediet ranks.
//!
//! A record set stands in one document whose format name is
//! suitediet-records/1. Reading is exact. A missing field or an
//! unexpected field or a duplicated name or a repeated key is
//! refused rather than guessed, because the plan is only as honest
//! as the records it rests on.

use std::path::Path;

use crate::bigint::BigInt;
use crate::errors::SuitedietError;
use crate::jsonvalue::{parse_json, JsonValue, ParseFailure};
use crate::pytext::path_display;
use crate::render::{repr_string, repr_value};

pub const RECORDS_FORMAT: &str = "suitediet-records/1";
pub const QUARANTINE_FORMAT: &str = "suitediet-quarantine/1";

const DOCUMENT_FIELDS: [&str; 3] = ["format", "suite", "tests"];
const RECORD_FIELDS: [&str; 3] = ["caught", "name", "runtime_ms"];

/// One measured test record, held in the normalised form with its
/// caught list sorted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestRecord {
    pub name: String,
    pub runtime_ms: BigInt,
    pub caught: Vec<String>,
}

/// A record set, held with its tests sorted by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordSet {
    pub suite: String,
    pub tests: Vec<TestRecord>,
}

impl RecordSet {
    pub fn record_named(&self, name: &str) -> Option<&TestRecord> {
        self.tests.iter().find(|test| test.name == name)
    }
}

/// A quarantine record, carrying the full record of every cleared
/// test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quarantine {
    pub suite: String,
    pub tests: Vec<TestRecord>,
}

fn read_json(text: &str, what: &str) -> Result<JsonValue, SuitedietError> {
    parse_json(text).map_err(|failure| match failure {
        ParseFailure::Invalid(message) => {
            SuitedietError::new(format!("the {what} is not valid JSON: {message}"))
        }
        ParseFailure::Boundary => SuitedietError::new(format!("the {what} is not valid JSON")),
        ParseFailure::DuplicateKey(key) => SuitedietError::new(format!(
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

fn read_test_records(
    raw: &JsonValue,
    what: &str,
    allow_empty: bool,
) -> Result<Vec<TestRecord>, SuitedietError> {
    let entries = match raw.as_list() {
        Some(entries) => entries,
        None => {
            return Err(SuitedietError::new(format!(
                "the {what} tests field is not a list"
            )));
        }
    };
    if entries.is_empty() && !allow_empty {
        return Err(SuitedietError::new(format!(
            "the {what} tests list is empty"
        )));
    }
    let mut records: Vec<TestRecord> = Vec::new();
    let mut seen_names: Vec<String> = Vec::new();
    for entry in entries {
        let fields = match entry.as_object() {
            Some(fields) => fields,
            None => {
                return Err(SuitedietError::new(format!(
                    "a test record in the {what} is not an object"
                )));
            }
        };
        let keys: Vec<&str> = fields.iter().map(|(key, _)| key.as_str()).collect();
        let mut sorted_keys = keys.clone();
        sorted_keys.sort();
        if sorted_keys.as_slice() != &RECORD_FIELDS[..] {
            return Err(SuitedietError::new(format!(
                "a test record in the {what} carries fields {} rather than {}",
                fields_spelling(&keys),
                expected_fields_spelling(&RECORD_FIELDS),
            )));
        }
        let name = match entry.get("name") {
            Some(JsonValue::Text(text)) if !text.is_empty() => text.clone(),
            _ => {
                return Err(SuitedietError::new(format!(
                    "a test record in the {what} carries an empty or non text name"
                )));
            }
        };
        if seen_names.iter().any(|seen| seen == &name) {
            return Err(SuitedietError::new(format!(
                "the {what} lists the test name {name} twice"
            )));
        }
        seen_names.push(name.clone());
        let runtime = match entry.get("runtime_ms") {
            Some(JsonValue::Int(value)) => BigInt::from_i64(*value),
            Some(JsonValue::BigInt(text)) => {
                BigInt::from_decimal(text).expect("a stored integer token reads back")
            }
            _ => {
                return Err(SuitedietError::new(format!(
                    "the test {name} carries a runtime that is not a positive whole number of milliseconds"
                )));
            }
        };
        if !runtime.is_positive() {
            return Err(SuitedietError::new(format!(
                "the test {name} carries a runtime that is not a positive whole number of milliseconds"
            )));
        }
        let caught_raw = match entry.get("caught") {
            Some(JsonValue::List(items)) => items,
            _ => {
                return Err(SuitedietError::new(format!(
                    "the test {name} carries a caught field that is not a list"
                )));
            }
        };
        let mut caught: Vec<String> = Vec::new();
        for defect in caught_raw {
            match defect {
                JsonValue::Text(text) if !text.is_empty() => {
                    caught.push(text.clone());
                }
                _ => {
                    return Err(SuitedietError::new(format!(
                        "the test {name} catches an empty or non text defect identifier"
                    )));
                }
            }
        }
        let mut deduped = caught.clone();
        deduped.sort();
        deduped.dedup();
        if deduped.len() != caught.len() {
            return Err(SuitedietError::new(format!(
                "the test {name} lists one defect identifier twice"
            )));
        }
        caught.sort();
        records.push(TestRecord {
            name,
            runtime_ms: runtime,
            caught,
        });
    }
    records.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(records)
}

fn read_document(
    text: &str,
    format_name: &str,
    what: &str,
    allow_empty: bool,
) -> Result<(String, Vec<TestRecord>), SuitedietError> {
    let document = read_json(text, what)?;
    let fields = match document.as_object() {
        Some(fields) => fields,
        None => {
            return Err(SuitedietError::new(format!("the {what} is not an object")));
        }
    };
    let keys: Vec<&str> = fields.iter().map(|(key, _)| key.as_str()).collect();
    let mut sorted_keys = keys.clone();
    sorted_keys.sort();
    if sorted_keys.as_slice() != &DOCUMENT_FIELDS[..] {
        return Err(SuitedietError::new(format!(
            "the {what} carries fields {} rather than {}",
            fields_spelling(&keys),
            expected_fields_spelling(&DOCUMENT_FIELDS),
        )));
    }
    let format_value = document.get("format").expect("the field stands");
    let format_matches = matches!(
        format_value,
        JsonValue::Text(text) if text == format_name
    );
    if !format_matches {
        return Err(SuitedietError::new(format!(
            "the {what} carries the format name {} rather than {}",
            repr_value(format_value),
            repr_string(format_name),
        )));
    }
    let suite = match document.get("suite") {
        Some(JsonValue::Text(text)) if !text.is_empty() => text.clone(),
        _ => {
            return Err(SuitedietError::new(format!(
                "the {what} carries an empty or non text suite name"
            )));
        }
    };
    let tests = read_test_records(
        document.get("tests").expect("the field stands"),
        what,
        allow_empty,
    )?;
    Ok((suite, tests))
}

/// Read a records document from its text.
pub fn read_records_text(text: &str) -> Result<RecordSet, SuitedietError> {
    let (suite, tests) = read_document(text, RECORDS_FORMAT, "records document", false)?;
    Ok(RecordSet { suite, tests })
}

/// Read a remaining records document from its text, accepting an
/// empty tests list as a true reading of an emptied suite.
pub fn read_remaining_text(text: &str) -> Result<RecordSet, SuitedietError> {
    let (suite, tests) = read_document(text, RECORDS_FORMAT, "remaining records document", true)?;
    Ok(RecordSet { suite, tests })
}

/// Read a quarantine record from its text.
pub fn read_quarantine_text(text: &str) -> Result<Quarantine, SuitedietError> {
    let (suite, tests) = read_document(text, QUARANTINE_FORMAT, "quarantine record", true)?;
    Ok(Quarantine { suite, tests })
}

/// Read a file as UTF-8 text with its line endings translated to
/// the line feed character, as the Python file reading delivers
/// it. The label names the document in the refusals and the path
/// stands as written.
pub fn read_file_text(path: &Path, what: &str) -> Result<String, SuitedietError> {
    let display = path_display(path);
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(SuitedietError::new(format!(
                "the {what} at {display} does not exist"
            )));
        }
        Err(error) => {
            return Err(SuitedietError::new(format!(
                "the {what} at {display} cannot be read: {error}"
            )));
        }
    };
    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => {
            return Err(SuitedietError::new(format!("the {what} is not valid JSON")));
        }
    };
    Ok(text.replace("\r\n", "\n").replace('\r', "\n"))
}

/// Read a records document from a local file.
pub fn read_records(path: &Path) -> Result<RecordSet, SuitedietError> {
    let text = read_file_text(path, "records document")?;
    read_records_text(&text)
}

/// Read a remaining records document from a local file.
pub fn read_remaining(path: &Path) -> Result<RecordSet, SuitedietError> {
    let text = read_file_text(path, "remaining records document")?;
    read_remaining_text(&text)
}

/// Read a quarantine record from a local file.
pub fn read_quarantine(path: &Path) -> Result<Quarantine, SuitedietError> {
    let text = read_file_text(path, "quarantine record")?;
    read_quarantine_text(&text)
}

/// A record as a written document value.
pub fn record_document(record: &TestRecord) -> JsonValue {
    JsonValue::object(vec![
        ("name", JsonValue::Text(record.name.clone())),
        ("runtime_ms", bigint_json(&record.runtime_ms)),
        (
            "caught",
            JsonValue::List(
                record
                    .caught
                    .iter()
                    .map(|defect| JsonValue::Text(defect.clone()))
                    .collect(),
            ),
        ),
    ])
}

/// A big integer as a written document value, an ordinary integer
/// when it fits and its exact digits when it does not.
pub fn bigint_json(value: &BigInt) -> JsonValue {
    match value.to_i64() {
        Some(integer) => JsonValue::Int(integer),
        None => JsonValue::BigInt(value.to_decimal_string()),
    }
}

/// A record set as a written records document.
pub fn records_document(record_set: &RecordSet) -> JsonValue {
    JsonValue::object(vec![
        ("format", JsonValue::Text(RECORDS_FORMAT.to_string())),
        ("suite", JsonValue::Text(record_set.suite.clone())),
        (
            "tests",
            JsonValue::List(record_set.tests.iter().map(record_document).collect()),
        ),
    ])
}

/// A quarantine record as a written document.
pub fn quarantine_document(quarantine: &Quarantine) -> JsonValue {
    JsonValue::object(vec![
        ("format", JsonValue::Text(QUARANTINE_FORMAT.to_string())),
        ("suite", JsonValue::Text(quarantine.suite.clone())),
        (
            "tests",
            JsonValue::List(quarantine.tests.iter().map(record_document).collect()),
        ),
    ])
}
