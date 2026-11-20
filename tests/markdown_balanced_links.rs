//! The pins of slice 17, the markdown rendering of
//! names, the balanced link spelling, over synthetic
//! records the suite writes itself. Each pin plans a
//! record set carrying the name it names and asserts
//! the markdown the plan writes, the name reading as
//! itself where the markdown is rendered. Eight pins
//! fail on the head this slice branches from, where
//! the rendering reads a link spelling's closing
//! bracket as the first closing bracket standing
//! after the opening, and five stand on both heads
//! at the lines MP-2 and MP-3 draw.

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
          {{\"name\": \"alpha\", \"runtime_ms\": 100, \"caught\": [\"d1\"]}},
          {{\"name\": \"beta\", \"runtime_ms\": 50, \"caught\": [\"d1\"]}}
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
fn test_a_bracket_inside_a_link_spelling_in_a_ranking_name_stands_escaped_at_the_outer_opening() {
    let md = rendered(&records_with_name("\"[a[b]](x)\""), "100000");
    let row = ranking_row(&md, "a[b]");
    assert!(
        row.contains("| \\[a[b]](x) |"),
        "the cell carries the outer opening escaped: {row}"
    );
}

#[test]
fn test_a_bracket_inside_a_link_text_in_a_ranking_name_stands_escaped_at_the_outer_opening() {
    let md = rendered(&records_with_name("\"[a[b]c](x)\""), "100000");
    let row = ranking_row(&md, "a[b]c");
    assert!(
        row.contains("| \\[a[b]c](x) |"),
        "the cell carries the outer opening escaped: {row}"
    );
}

#[test]
fn test_a_doubled_bracket_spelling_in_a_ranking_name_stands_escaped_at_the_outer_opening() {
    let md = rendered(&records_with_name("\"[[a]](x)\""), "100000");
    let row = ranking_row(&md, "[a]");
    assert!(
        row.contains("| \\[[a]](x) |"),
        "the cell carries the outer opening escaped: {row}"
    );
}

#[test]
fn test_an_image_spelling_with_a_bracket_inside_in_a_ranking_name_stands_escaped_at_its_opening() {
    let md = rendered(&records_with_name("\"![a[b]](x)\""), "100000");
    let row = ranking_row(&md, "a[b]");
    assert!(
        row.contains("| !\\[a[b]](x) |"),
        "the cell carries the image opening escaped: {row}"
    );
}

#[test]
fn test_a_link_spelling_under_an_unbalanced_bracket_in_a_ranking_name_stands_escaped_at_both_openings(
) {
    let md = rendered(&records_with_name("\"[a[b](x)\""), "100000");
    let row = ranking_row(&md, "b](x)");
    assert!(
        row.contains("| \\[a\\[b](x) |"),
        "the cell carries both openings escaped: {row}"
    );
}

#[test]
fn test_a_link_spelling_inside_a_bracketed_name_stands_escaped_at_both_openings() {
    let md = rendered(&records_with_name("\"[a [b](x) c]\""), "100000");
    let row = ranking_row(&md, "b](x)");
    assert!(
        row.contains("| \\[a \\[b](x) c] |"),
        "the cell carries both openings escaped: {row}"
    );
}

#[test]
fn test_a_bracketed_link_spelling_inside_a_ranking_name_stands_escaped_at_its_opening() {
    let md = rendered(&records_with_name("\"x [a [b] c](y) z\""), "100000");
    let row = ranking_row(&md, "[b]");
    assert!(
        row.contains("| x \\[a [b] c](y) z |"),
        "the cell carries the spelling's opening escaped: {row}"
    );
}

#[test]
fn test_a_thrice_nested_link_spelling_in_a_ranking_name_stands_escaped_at_every_opening() {
    let md = rendered(&records_with_name("\"[a [b [c](x)] d]\""), "100000");
    let row = ranking_row(&md, "[c]");
    assert!(
        row.contains("| \\[a \\[b \\[c](x)] d] |"),
        "the cell carries every opening escaped: {row}"
    );
}

#[test]
fn test_a_balanced_link_spelling_in_a_suite_name_stands_escaped_in_the_prose() {
    let md = rendered(&records_with_suite("\"[a[b]](x)\""), "100000");
    let line = md
        .lines()
        .find(|line| line.starts_with("The suite "))
        .expect("the suite sentence stands")
        .to_string();
    assert!(
        line.contains("The suite \\[a[b]](x) under a budget"),
        "the prose carries the outer opening escaped: {line}"
    );
}

#[test]
fn test_a_bracket_standing_without_the_whole_spelling_in_a_ranking_name_stands_as_it_stands() {
    let md = rendered(&records_with_name("\"a[0]\""), "100000");
    let row = ranking_row(&md, "a[0]");
    assert!(
        row.contains("| a[0] |"),
        "the cell carries the bracket raw: {row}"
    );
}

#[test]
fn test_a_bracket_closed_without_a_parenthesis_in_a_ranking_name_stands_as_it_stands() {
    let md = rendered(&records_with_name("\"[a]b](c)\""), "100000");
    let row = ranking_row(&md, "[a]b");
    assert!(
        row.contains("| [a]b](c) |"),
        "the cell carries the brackets raw: {row}"
    );
}

#[test]
fn test_a_whole_link_spelling_in_a_ranking_name_stands_escaped_at_its_opening() {
    let md = rendered(
        &records_with_name("\"[docs](https://example.com)\""),
        "100000",
    );
    let row = ranking_row(&md, "docs");
    assert!(
        row.contains("| \\[docs](https://example.com) |"),
        "the cell carries the opening escaped: {row}"
    );
}

#[test]
fn test_an_unbalanced_bracket_spelling_in_a_ranking_name_stands_as_it_stands() {
    let md = rendered(&records_with_name("\"[a[b]c\""), "100000");
    let row = ranking_row(&md, "a[b]c");
    assert!(
        row.contains("| [a[b]c |"),
        "the cell carries the brackets raw: {row}"
    );
}
