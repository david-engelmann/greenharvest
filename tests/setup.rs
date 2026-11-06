//! The seed pins of the Python setup suite, re-expressed in
//! Rust under their own names, reading the same files for the
//! same sentences. The gate sentence stands at the wording
//! the closing slice of the port gives it.

use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).expect("a seed file stands on disk")
}

#[test]
fn test_seed_files_present() {
    let readme = read("README.md");
    let licence = read("LICENSE");
    let gitignore = read(".gitignore");
    assert!(readme.contains("# greenharvest"));
    assert!(readme.contains("Suite dieting with a safety proof"));
    assert!(readme.contains("## Licence"));
    assert!(readme.contains("Closed rounds stand at 3 of 100"));
    assert!(licence.contains("MIT License"));
    assert!(gitignore.contains("__pycache__/"));
    assert!(gitignore.contains(".env"));
}

#[test]
fn test_agents_template_and_annex() {
    let agents = read("AGENTS.md");
    let marker = "<!-- repo annex below this line -->";
    assert!(agents.starts_with("# AGENTS.md"));
    assert!(agents.contains(
        "This repo is one of the repos in David Engelmann's 39 repo storefront perfection program"
    ));
    assert!(agents.contains(marker));
    let annex = agents
        .split(marker)
        .nth(1)
        .expect("the annex stands below its marker");
    assert!(annex.contains(
        "Ostensible asks whether a test can fail, greenharvest asks which tests earn their runtime"
    ));
    assert!(annex.contains("The tool deletes nothing"));
    assert!(annex.contains("reversible quarantine"));
    assert!(annex.contains("synthetic fixtures only and no network in the tool or the tests"));
    assert!(annex.contains("The local gate is cargo test"));
    assert!(annex.contains("closed round count stands in the README and moves only at a closure"));
}
