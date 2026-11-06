//! Recorded conformance for the command line, the closing
//! slice's battery.
//!
//! Every case in tests/conformance/cli_cases.json was recorded
//! from the landed Python command line run as a black box, and
//! the boundary cases carry the port's stated answer where the
//! Python crashes or where a stated limit of the port spells a
//! message differently, each marked in the recording with the
//! Python outcome standing beside it. The recordings are
//! generated and never hand edited. The census of every
//! section is asserted here as well as carried in the
//! recording, so a regenerated recording that quietly drops a
//! case fails the suite.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use greenharvest::jsonvalue::{parse_json, JsonValue};

const TOTAL_CASES: usize = 99;
const BOUNDARY_CASES: usize = 8;

fn expected_sections() -> Vec<(&'static str, usize)> {
    vec![
        ("behaviours", 9),
        ("arguments", 56),
        ("refusals", 20),
        ("limits", 14),
    ]
}

const USAGE_TOP: &str = "usage: greenharvest {plan,restore} ...\n";
const USAGE_PLAN: &str = "usage: greenharvest plan RECORDS --budget-ms BUDGET --out-dir OUT_DIR\n";
const USAGE_RESTORE: &str = "usage: greenharvest restore REMAINING QUARANTINE --out OUT\n";

fn usage_line(marker: &str) -> &'static str {
    match marker {
        "top" => USAGE_TOP,
        "plan" => USAGE_PLAN,
        "restore" => USAGE_RESTORE,
        _ => panic!("a usage marker names a parser, not {marker}"),
    }
}

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_greenharvest"))
}

/// A scratch directory carrying the process id in its name,
/// removed when the guard drops.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let root =
            std::env::temp_dir().join(format!("greenharvest-cli-{}-{}", std::process::id(), tag));
        if root.exists() {
            std::fs::remove_dir_all(&root).expect("a stale scratch dir clears");
        }
        std::fs::create_dir_all(&root).expect("a scratch dir is created");
        Scratch { root }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn load() -> JsonValue {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/conformance/cli_cases.json");
    let text = std::fs::read_to_string(&path).expect("the recordings stand on disk");
    parse_json(&text).expect("the recordings parse")
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

fn base64_decode(text: &str) -> Vec<u8> {
    fn value(byte: u8) -> Option<u8> {
        match byte {
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::new();
    let mut buffer: u32 = 0;
    let mut held = 0;
    for byte in text.bytes() {
        if byte == b'=' {
            break;
        }
        let digit = value(byte).expect("a recorded word is base64");
        buffer = (buffer << 6) | digit as u32;
        held += 6;
        if held >= 8 {
            held -= 8;
            out.push((buffer >> held) as u8);
        }
    }
    out
}

/// The bytes of one recorded argument word, a text standing as
/// itself and a byte word standing in base64.
fn word_bytes(value: &JsonValue) -> Vec<u8> {
    match value {
        JsonValue::Text(text) => text.clone().into_bytes(),
        JsonValue::Object(_) => base64_decode(field_text(value, "b64")),
        _ => panic!("an argument word stands as text or bytes"),
    }
}

fn os_word(bytes: &[u8]) -> OsString {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        OsString::from_vec(bytes.to_vec())
    }
    #[cfg(not(unix))]
    {
        OsString::from(String::from_utf8_lossy(bytes).into_owned())
    }
}

/// The files standing under a directory, as relative paths to
/// their bytes, with a directory standing as its path and a
/// trailing slash over empty bytes.
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
            .expect("a case dir reads")
            .map(|entry| entry.expect("a case entry reads").path())
            .collect();
        entries.sort();
        for entry in entries {
            let rel = entry
                .strip_prefix(base)
                .expect("an entry stands under the case dir")
                .to_string_lossy()
                .into_owned();
            if entry.is_dir() {
                out.insert(format!("{rel}/"), Vec::new());
                walk(base, &entry, out);
            } else {
                out.insert(rel, std::fs::read(&entry).expect("a case file reads"));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn expected_snapshot(case: &JsonValue) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let files = field(case, "final_files");
    if let JsonValue::Object(pairs) = files {
        for (name, value) in pairs {
            let bytes = match value {
                JsonValue::Text(text) => text.clone().into_bytes(),
                _ => panic!("a recorded file stands as text"),
            };
            out.insert(name.clone(), bytes);
        }
    } else {
        panic!("the final files stand as an object");
    }
    out
}

/// The expected stream bytes of one step, the recorded text or
/// the usage line of the parser the recording names.
fn expected_stream(expect: &JsonValue, stream: &str) -> Vec<u8> {
    match field(expect, stream) {
        JsonValue::Text(text) => text.clone().into_bytes(),
        JsonValue::Null => usage_line(field_text(expect, "usage")).as_bytes().to_vec(),
        _ => panic!("a stream stands as text or as a usage marker"),
    }
}

fn run_case(case: &JsonValue) -> Result<(), String> {
    let label = format!(
        "{} / {}",
        field_text(case, "section"),
        field_text(case, "name")
    );
    let scratch = Scratch::new(&format!("case-{}", label.len()));
    let root = scratch.root.clone();
    if let JsonValue::Object(pairs) = field(case, "files") {
        for (name, value) in pairs {
            let text = match value {
                JsonValue::Text(text) => text,
                _ => return Err(format!("{label}: an input file stands as text")),
            };
            let path = root.join(name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|err| format!("{label}: staging a parent failed: {err}"))?;
            }
            std::fs::write(&path, text)
                .map_err(|err| format!("{label}: staging an input failed: {err}"))?;
        }
    }
    if let JsonValue::List(dirs) = field(case, "dirs") {
        for dir in dirs {
            let name = dir
                .as_text()
                .ok_or_else(|| format!("{label}: a staged directory stands as text"))?;
            std::fs::create_dir_all(root.join(name))
                .map_err(|err| format!("{label}: staging a directory failed: {err}"))?;
        }
    }
    let steps = field(case, "steps")
        .as_list()
        .ok_or_else(|| format!("{label}: the steps stand as a list"))?;
    for (index, step) in steps.iter().enumerate() {
        let argv: Vec<OsString> = field(step, "argv")
            .as_list()
            .ok_or_else(|| format!("{label}: an argv stands as a list"))?
            .iter()
            .map(|word| os_word(&word_bytes(word)))
            .collect();
        let output = Command::new(binary())
            .args(&argv)
            .current_dir(&root)
            .output()
            .map_err(|err| format!("{label}: step {index} did not run: {err}"))?;
        let expect = field(step, "expect");
        let wanted_exit = match field(expect, "exit") {
            JsonValue::Int(value) => *value,
            _ => return Err(format!("{label}: an exit stands as a whole number")),
        };
        if output.status.code() != Some(wanted_exit as i32) {
            return Err(format!(
                "{label}: step {index} exit {} stands against the recorded {}",
                output.status.code().unwrap_or(-1),
                wanted_exit
            ));
        }
        if output.stdout != expected_stream(expect, "stdout") {
            return Err(format!(
                "{label}: step {index} stdout {:?} stands against the recording",
                String::from_utf8_lossy(&output.stdout)
            ));
        }
        if output.stderr != expected_stream(expect, "stderr") {
            return Err(format!(
                "{label}: step {index} stderr {:?} stands against the recording",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }
    let standing = snapshot(&root);
    let recorded = expected_snapshot(case);
    if standing != recorded {
        let mut names: Vec<&String> = standing.keys().chain(recorded.keys()).collect();
        names.sort();
        names.dedup();
        for name in names {
            if standing.get(name) != recorded.get(name) {
                return Err(format!(
                    "{label}: the file {name} stands against the recording"
                ));
            }
        }
        return Err(format!(
            "{label}: the written files stand against the recording"
        ));
    }
    Ok(())
}

#[test]
fn test_cli_battery_matches_the_python_record() {
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
    let all = document
        .get("cases")
        .and_then(JsonValue::as_list)
        .expect("the recordings carry their cases");
    assert_eq!(all.len(), TOTAL_CASES, "the case list matches its count");
    let mut computed: BTreeMap<String, usize> = BTreeMap::new();
    let mut boundary = 0;
    for case in all {
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
            "the {section} section stands at its recorded count"
        );
    }
    let mut failures = Vec::new();
    for case in all {
        if let Err(problem) = run_case(case) {
            failures.push(problem);
        }
    }
    assert!(
        failures.is_empty(),
        "the battery disagrees with the record:\n{}",
        failures.join("\n")
    );
}

/// The budget word carrying a byte that is not UTF-8 text is
/// spelled in its refusal by its Python representation, the
/// surrogate escape of the byte standing in the wording and
/// never a replacement character.
#[test]
fn test_a_budget_word_carrying_a_non_utf8_byte_is_spelled_by_its_representation() {
    let scratch = Scratch::new("budget-byte");
    let records = scratch.root.join("records.json");
    std::fs::write(
        &records,
        "{\"format\": \"suitediet-records/1\", \"suite\": \"s\", \"tests\": [\
         {\"name\": \"test_a\", \"runtime_ms\": 10, \"caught\": []}]}\n",
    )
    .expect("the records document is staged");
    let mut budget = b"1".to_vec();
    budget.push(0xFF);
    budget.extend_from_slice(b"2");
    let output = Command::new(binary())
        .args([
            OsString::from("plan"),
            records.clone().into_os_string(),
            OsString::from("--budget-ms"),
            os_word(&budget),
            OsString::from("--out-dir"),
            OsString::from("out"),
        ])
        .current_dir(&scratch.root)
        .output()
        .expect("the binary runs");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        output.stderr,
        b"error: the budget '1\\udcff2' is not a whole number of milliseconds\n".to_vec()
    );
    assert!(
        !scratch.root.join("out").exists(),
        "a refused budget writes nothing"
    );
}
