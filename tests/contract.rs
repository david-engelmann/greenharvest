//! The contract audit of the closing slice.
//!
//! The name checker of the Python contract suite stands
//! re-expressed here under its new name, reading this repo's
//! contract and asserting that every test name it carries is
//! disposed in the mapping, and that every standing the
//! mapping claims stands in the tree. The dash pin is this
//! slice's own audit pin.

use std::path::PathBuf;

use greenharvest::jsonvalue::{parse_json, JsonValue};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).expect("a contract file stands on disk")
}

/// Every test name the contract carries, read off its text
/// the way the Python checker read them.
fn contract_names() -> Vec<String> {
    let contract = read("CONTRACT.md");
    let mut names: Vec<String> = Vec::new();
    for token in
        contract.split(|ch: char| !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_'))
    {
        if token.starts_with("test_") && token.len() > 5 && !names.contains(&token.to_string()) {
            names.push(token.to_string());
        }
    }
    names
}

struct Disposition {
    name: String,
    kind: String,
    rest: String,
}

fn mapping() -> Vec<Disposition> {
    let text = read("tests/contract_name_map.txt");
    let mut held = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(3, ' ');
        let name = parts.next().expect("a mapping line names a test");
        let kind = parts.next().expect("a mapping line names a disposition");
        let rest = parts.next().unwrap_or("");
        held.push(Disposition {
            name: name.to_string(),
            kind: kind.to_string(),
            rest: rest.to_string(),
        });
    }
    held
}

/// Whether a Rust test of the given name stands in a file, as
/// a function of that name.
fn rust_test_stands(rel: &str, test_name: &str) -> bool {
    let text = read(rel);
    text.contains(&format!("fn {test_name}("))
}

/// The case objects of a recordings file, standing in its
/// case list.
fn recording_cases(rel: &str) -> Vec<JsonValue> {
    let text = read(rel);
    let document = parse_json(&text).expect("a recordings file parses");
    document
        .get("cases")
        .and_then(JsonValue::as_list)
        .cloned()
        .unwrap_or_default()
}

fn recording_case_stands(rel: &str, case_name: &str) -> bool {
    recording_cases(rel)
        .iter()
        .any(|case| case.get("name").and_then(JsonValue::as_text) == Some(case_name))
}

fn recording_section_stands(rel: &str, section: &str) -> bool {
    let text = read(rel);
    let document = parse_json(&text).expect("a recordings file parses");
    match document.get("sections") {
        Some(JsonValue::Object(pairs)) => pairs.iter().any(|(name, _)| name == section),
        _ => false,
    }
}

#[test]
fn test_every_contract_name_is_disposed() {
    let names = contract_names();
    let held = mapping();
    for name in &names {
        let lines: Vec<&Disposition> = held.iter().filter(|line| &line.name == name).collect();
        assert!(
            !lines.is_empty(),
            "the contract name {name} stands undisposed"
        );
        for line in lines {
            let mut parts = line.rest.splitn(2, ' ');
            let file = parts.next().expect("a disposition names its file");
            let target = parts.next().unwrap_or("");
            let stands = match line.kind.as_str() {
                "rust-test" => rust_test_stands(file, name),
                "re-expressed" => rust_test_stands(file, target),
                "recording-case" => recording_case_stands(file, target),
                "recording-section" => recording_section_stands(file, target),
                other => panic!("the disposition kind {other} is not one the audit reads"),
            };
            assert!(
                stands,
                "the disposition of {name} claims a standing that is not in the tree: {} {}",
                line.kind, line.rest
            );
        }
    }
}

#[test]
fn test_contract_has_no_forbidden_dashes() {
    for rel in ["CONTRACT.md", "README.md", "AGENTS.md"] {
        let text = read(rel);
        assert!(!text.contains('\u{2014}'), "{rel} carries an em dash");
        assert!(!text.contains('\u{2013}'), "{rel} carries an en dash");
    }
}
