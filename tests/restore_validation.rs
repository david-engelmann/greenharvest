//! The pins of slice 8, the restore validation, over
//! synthetic fixtures the suite writes itself and over the
//! binary's validate-restore door.

use std::path::PathBuf;
use std::process::Command;

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_greenharvest"))
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let path = std::env::temp_dir().join(format!(
            "greenharvest-restore-validation-{}-{tag}",
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

/// The remaining records of the suite, two tests standing
/// after a plan quarantined the third.
const REMAINING: &str = r#"{
  "format": "suitediet-records/1",
  "suite": "restore validation suite",
  "tests": [
    {"name": "beta", "runtime_ms": 50, "caught": ["d1"]},
    {"name": "gamma", "runtime_ms": 10, "caught": []}
  ]
}"#;

/// The quarantine record of that plan, the one test it
/// removed with the record it was removed under.
const QUARANTINE: &str = r#"{
  "format": "suitediet-quarantine/1",
  "suite": "restore validation suite",
  "tests": [
    {"name": "alpha", "runtime_ms": 100, "caught": ["d1", "d2"]}
  ]
}"#;

/// The restored set the pair restores to, spelled compactly
/// with its keys shuffled and its tests standing out of
/// name order, so the pin proves the comparison stands
/// over values and not over spelling.
const RESTORED: &str = r#"{"tests": [{"caught": ["d1"], "runtime_ms": 50, "name": "beta"}, {"name": "gamma", "runtime_ms": 10, "caught": []}, {"name": "alpha", "runtime_ms": 100, "caught": ["d1", "d2"]}], "suite": "restore validation suite", "format": "suitediet-records/1"}"#;

const DISAGREEMENT: &str =
    "the restored record set disagrees with the remaining records and the quarantine record";

fn validate(
    scratch: &Scratch,
    remaining: &str,
    quarantine: &str,
    restored: &str,
) -> std::process::Output {
    let remaining_path = scratch.write("remaining.json", remaining);
    let quarantine_path = scratch.write("quarantine.json", quarantine);
    let restored_path = scratch.write("restored.json", restored);
    Command::new(binary())
        .arg("validate-restore")
        .arg(&remaining_path)
        .arg(&quarantine_path)
        .arg(&restored_path)
        .output()
        .expect("the door runs")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn test_a_genuine_restored_set_validates() {
    let scratch = Scratch::new("genuine");
    let output = validate(&scratch, REMAINING, QUARANTINE, RESTORED);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(output.stdout.is_empty());
}

#[test]
fn test_a_restored_set_with_an_altered_runtime_is_refused() {
    let scratch = Scratch::new("runtime");
    let altered = RESTORED.replace("\"runtime_ms\": 50", "\"runtime_ms\": 51");
    let output = validate(&scratch, REMAINING, QUARANTINE, &altered);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains(DISAGREEMENT),
        "{}",
        stderr(&output)
    );
}

#[test]
fn test_a_restored_set_with_an_altered_caught_list_is_refused() {
    let scratch = Scratch::new("caught");
    let altered = RESTORED.replace(
        "{\"name\": \"alpha\", \"runtime_ms\": 100, \"caught\": [\"d1\", \"d2\"]}",
        "{\"name\": \"alpha\", \"runtime_ms\": 100, \"caught\": [\"d1\"]}",
    );
    let output = validate(&scratch, REMAINING, QUARANTINE, &altered);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains(DISAGREEMENT),
        "{}",
        stderr(&output)
    );
}

#[test]
fn test_a_restored_set_missing_a_quarantined_test_is_refused() {
    let scratch = Scratch::new("missing");
    let missing = r#"{"format": "suitediet-records/1", "suite": "restore validation suite", "tests": [{"name": "beta", "runtime_ms": 50, "caught": ["d1"]}, {"name": "gamma", "runtime_ms": 10, "caught": []}]}"#;
    let output = validate(&scratch, REMAINING, QUARANTINE, missing);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains(DISAGREEMENT),
        "{}",
        stderr(&output)
    );
}

#[test]
fn test_a_restored_set_with_an_extra_test_is_refused() {
    let scratch = Scratch::new("extra");
    let extra = RESTORED.replace(
        "{\"name\": \"gamma\"",
        "{\"name\": \"delta\", \"runtime_ms\": 5, \"caught\": [\"d4\"]}, {\"name\": \"gamma\"",
    );
    assert!(extra != RESTORED);
    let output = validate(&scratch, REMAINING, QUARANTINE, &extra);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains(DISAGREEMENT),
        "{}",
        stderr(&output)
    );
}

#[test]
fn test_a_restored_set_naming_the_wrong_suite_is_refused() {
    let scratch = Scratch::new("suite");
    let wrong = RESTORED.replace("restore validation suite", "another suite");
    let output = validate(&scratch, REMAINING, QUARANTINE, &wrong);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains(DISAGREEMENT),
        "{}",
        stderr(&output)
    );
}

#[test]
fn test_a_restored_set_from_another_quarantine_is_refused() {
    let scratch = Scratch::new("another");
    let other_restored = r#"{"format": "suitediet-records/1", "suite": "restore validation suite", "tests": [{"name": "beta", "runtime_ms": 50, "caught": ["d1"]}, {"name": "delta", "runtime_ms": 5, "caught": ["d4"]}, {"name": "gamma", "runtime_ms": 10, "caught": []}]}"#;
    let output = validate(&scratch, REMAINING, QUARANTINE, other_restored);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains(DISAGREEMENT),
        "{}",
        stderr(&output)
    );
}

#[test]
fn test_a_pair_that_cannot_stand_together_is_refused_in_the_restore_words() {
    let scratch = Scratch::new("collision");
    let remaining_with_alpha = r#"{
  "format": "suitediet-records/1",
  "suite": "restore validation suite",
  "tests": [
    {"name": "alpha", "runtime_ms": 100, "caught": ["d1", "d2"]},
    {"name": "beta", "runtime_ms": 50, "caught": ["d1"]}
  ]
}"#;
    let output = validate(&scratch, remaining_with_alpha, QUARANTINE, RESTORED);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output)
            .contains("the quarantined test alpha is still standing in the remaining records"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn test_an_empty_quarantine_restores_to_the_remaining_set_and_validates() {
    let scratch = Scratch::new("empty");
    let empty_quarantine = r#"{
  "format": "suitediet-quarantine/1",
  "suite": "restore validation suite",
  "tests": []
}"#;
    let output = validate(&scratch, REMAINING, empty_quarantine, REMAINING);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(output.stdout.is_empty());
}
