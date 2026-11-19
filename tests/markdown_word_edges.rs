//! The pins of slice 15, the markdown rendering of
//! names, the word edges, over synthetic records the
//! suite writes itself. Each pin plans a record set
//! carrying the name it names and asserts the markdown
//! the plan writes, the name reading as itself where
//! the markdown is rendered. Six pins fail on the head
//! this slice branches from, where the rendering
//! writes a word edge underscore raw, and two stand on
//! both heads at the lines MM-2 and ML-2 draw.

use greenharvest::{build_plan, markdown_text, parse_budget, read_records_text, report_document};

fn rendered(records: &str, budget: &str) -> String {
    let set = read_records_text(records).expect("the records read");
    let plan = build_plan(&set, &parse_budget(budget).expect("the budget parses"))
        .expect("the plan builds");
    markdown_text(&report_document(&plan))
}

fn records_with_name(name_json: &str) -> String {
    format!(
        "{{\"format\": \"suitediet-records/1\", \"suite\": \"probe\", \"tests\": [
          {{\"name\": \"alpha\", \"runtime_ms\": 100, \"caught\": [\"d1\"]}},
          {{\"name\": {}, \"runtime_ms\": 50, \"caught\": [\"d1\"]}}
        ]}}",
        name_json
    )
}

fn records_with_suite(suite_json: &str) -> String {
    format!(
        "{{\"format\": \"suitediet-records/1\", \"suite\": {}, \"tests\": [
          {{\"name\": \"alpha\", \"runtime_ms\": 100, \"caught\": [\"d1\"]}}
        ]}}",
        suite_json
    )
}

fn records_for_pack(name_json: &str, defect_json: &str) -> String {
    format!(
        "{{\"format\": \"suitediet-records/1\", \"suite\": \"probe\", \"tests\": [
          {{\"name\": \"cheap_keeper\", \"runtime_ms\": 10, \"caught\": [{}]}},
          {{\"name\": {}, \"runtime_ms\": 900, \"caught\": [{}]}}
        ]}}",
        defect_json, name_json, defect_json
    )
}

fn ranking_row(md: &str, marker: &str) -> String {
    md.lines()
        .find(|line| line.contains(marker))
        .expect("the row stands")
        .to_string()
}

fn pack_line(md: &str, marker: &str) -> String {
    md.lines()
        .find(|line| line.starts_with("- ") && line.contains(marker))
        .expect("the pack line stands")
        .to_string()
}

#[test]
fn test_a_word_edge_underscore_in_a_ranking_name_stands_escaped_in_its_cell() {
    let md = rendered(&records_with_name("\"test _fast_ case\""), "100000");
    let row = ranking_row(&md, "fast");
    assert!(
        row.contains("| test \\_fast\\_ case |"),
        "the cell carries the escaped word edge underscores: {row}"
    );
}

#[test]
fn test_an_underscore_beside_punctuation_in_a_ranking_name_stands_escaped_in_its_cell() {
    let md = rendered(&records_with_name("\"test (_fast_) case\""), "100000");
    let row = ranking_row(&md, "fast");
    assert!(
        row.contains("| test (\\_fast\\_) case |"),
        "the cell carries the escaped underscores beside the parentheses: {row}"
    );
}

#[test]
fn test_a_double_word_edge_underscore_in_a_pack_name_stands_escaped_in_the_prose() {
    let md = rendered(&records_for_pack("\"test __bold__ case\"", "\"d1\""), "100");
    let line = pack_line(&md, "bold");
    assert!(
        line.contains("- test \\_\\_bold\\_\\_ case is proposed for quarantine."),
        "the prose carries both escaped word edge runs: {line}"
    );
}

#[test]
fn test_a_word_edge_underscore_in_a_suite_name_stands_escaped_in_the_prose() {
    let md = rendered(&records_with_suite("\"the _suite_ files\""), "100000");
    let sentence = md
        .lines()
        .find(|line| line.contains("The suite"))
        .expect("the sentence stands");
    assert!(
        sentence.contains("The suite the \\_suite\\_ files under"),
        "the prose carries the escaped word edge underscores: {sentence}"
    );
}

#[test]
fn test_a_word_edge_underscore_in_a_defect_name_stands_escaped_in_the_prose() {
    let md = rendered(
        &records_for_pack("\"costly_test\"", "\"defect _one_ here\""),
        "100",
    );
    let line = pack_line(&md, "defect");
    assert!(
        line.contains("defect \\_one\\_ here is also caught by cheap_keeper"),
        "the prose carries the escaped word edge underscores: {line}"
    );
}

#[test]
fn test_an_underscore_beside_a_line_feed_in_a_ranking_name_stands_escaped_in_its_cell() {
    let md = rendered(&records_with_name("\"test\\n_fast_ case\""), "100000");
    let row = ranking_row(&md, "fast");
    assert!(
        row.contains("| test \\_fast\\_ case |"),
        "the cell carries the escaped underscores beside the written space: {row}"
    );
}

#[test]
fn test_an_interior_underscore_in_a_ranking_name_stands_as_it_stands() {
    let md = rendered(&records_with_name("\"plain_slow_test\""), "100000");
    let row = ranking_row(&md, "plain_slow_test");
    assert!(
        row.contains("| plain_slow_test |"),
        "the cell carries the interior underscores raw: {row}"
    );
}

#[test]
fn test_an_edge_underscore_in_a_ranking_name_stands_escaped_in_its_cell() {
    let md = rendered(&records_with_name("\"_fast_\""), "100000");
    let row = ranking_row(&md, "fast");
    assert!(
        row.contains("| \\_fast\\_ |"),
        "the cell carries the escaped edge underscores: {row}"
    );
}
