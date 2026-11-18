//! The pins of slice 14, the markdown rendering of
//! names, the edges and the link spelling, over
//! synthetic records the suite writes itself. Each pin
//! plans a record set carrying the name it names and
//! asserts the markdown the plan writes, the name
//! reading as itself where the markdown is rendered.
//! Six pins fail on the head this slice branches from,
//! where the rendering writes an edge underscore and a
//! link spelling raw, and two stand on both heads at
//! the lines ML-2 and ML-3 draw.

use greenharvest::{build_plan, markdown_text, parse_budget, read_records_text, report_document};

fn rendered(records: &str) -> String {
    let set = read_records_text(records).expect("the records read");
    let plan = build_plan(&set, &parse_budget("100000").expect("the budget parses"))
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

fn ranking_row(md: &str, marker: &str) -> String {
    md.lines()
        .find(|line| line.contains(marker))
        .expect("the row stands")
        .to_string()
}

#[test]
fn test_an_edge_underscore_in_a_ranking_name_stands_escaped_in_its_cell() {
    let md = rendered(&records_with_name("\"_fast_\""));
    let row = ranking_row(&md, "fast");
    assert!(
        row.contains("| \\_fast\\_ |"),
        "the cell carries the escaped edge underscores: {row}"
    );
}

#[test]
fn test_an_edge_underscore_in_a_suite_name_stands_escaped_in_the_prose() {
    let md = rendered(&records_with_suite("\"_suite_\""));
    let sentence = md
        .lines()
        .find(|line| line.contains("The suite"))
        .expect("the sentence stands");
    assert!(
        sentence.contains("The suite \\_suite\\_ under"),
        "the prose carries the escaped edge underscores: {sentence}"
    );
}

#[test]
fn test_a_double_edge_underscore_in_a_ranking_name_stands_escaped_in_its_cell() {
    let md = rendered(&records_with_name("\"__fast__\""));
    let row = ranking_row(&md, "fast");
    assert!(
        row.contains("| \\_\\_fast\\_\\_ |"),
        "the cell carries both escaped edge runs: {row}"
    );
}

#[test]
fn test_a_trailing_underscore_in_a_ranking_name_stands_escaped_in_its_cell() {
    let md = rendered(&records_with_name("\"fast_\""));
    let row = ranking_row(&md, "fast");
    assert!(
        row.contains("| fast\\_ |"),
        "the cell carries the escaped trailing underscore: {row}"
    );
}

#[test]
fn test_a_link_spelling_in_a_ranking_name_stands_escaped_in_its_cell() {
    let md = rendered(&records_with_name("\"[docs](https://example.com)\""));
    let row = ranking_row(&md, "docs");
    assert!(
        row.contains("| \\[docs](https://example.com) |"),
        "the cell carries the escaped opening bracket: {row}"
    );
}

#[test]
fn test_a_link_spelling_in_a_suite_name_stands_escaped_in_the_prose() {
    let md = rendered(&records_with_suite("\"[docs](https://example.com)\""));
    let sentence = md
        .lines()
        .find(|line| line.contains("The suite"))
        .expect("the sentence stands");
    assert!(
        sentence.contains("The suite \\[docs](https://example.com) under"),
        "the prose carries the escaped opening bracket: {sentence}"
    );
}

#[test]
fn test_an_interior_underscore_in_a_ranking_name_stands_as_it_stands() {
    let md = rendered(&records_with_name("\"test_fast_lane\""));
    let row = ranking_row(&md, "test_fast_lane");
    assert!(
        row.contains("| test_fast_lane |"),
        "the cell carries the interior underscores raw: {row}"
    );
}

#[test]
fn test_a_bracket_without_the_link_spelling_stands_as_it_stands() {
    let md = rendered(&records_with_name("\"a[0]\""));
    let row = ranking_row(&md, "a[0]");
    assert!(
        row.contains("| a[0] |"),
        "the cell carries the bracket raw: {row}"
    );
}
