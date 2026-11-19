//! The pins of slice 16, the markdown rendering of
//! names, the entity characters, over synthetic records
//! the suite writes itself. Each pin plans a record set
//! carrying the name it names and asserts the markdown
//! the plan writes, the name reading as itself where
//! the markdown is rendered. Six pins fail on the head
//! this slice branches from, where the rendering writes
//! a less than sign with a backslash before it and a
//! character reference raw, and two stand on both heads
//! at the lines MN-2 and MN-3 draw.

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
fn test_a_less_than_sign_opening_a_tag_in_a_ranking_name_stands_as_its_entity_spelling() {
    let md = rendered(&records_with_name("\"test <b>bold case\""), "100000");
    let row = ranking_row(&md, "bold");
    assert!(
        row.contains("| test &lt;b>bold case |"),
        "the cell carries the less than sign under its entity spelling: {row}"
    );
}

#[test]
fn test_a_less_than_sign_beside_a_space_in_a_ranking_name_stands_as_its_entity_spelling() {
    let md = rendered(&records_with_name("\"test < case\""), "100000");
    let row = ranking_row(&md, "test");
    assert!(
        row.contains("| test &lt; case |"),
        "the cell carries the less than sign under its entity spelling: {row}"
    );
}

#[test]
fn test_a_less_than_sign_beside_a_digit_in_a_ranking_name_stands_as_its_entity_spelling() {
    let md = rendered(&records_with_name("\"test <3 case\""), "100000");
    let row = ranking_row(&md, "test");
    assert!(
        row.contains("| test &lt;3 case |"),
        "the cell carries the less than sign under its entity spelling: {row}"
    );
}

#[test]
fn test_a_literal_entity_spelling_in_a_ranking_name_stands_with_its_ampersand_spelled() {
    let md = rendered(&records_with_name("\"test &lt; case\""), "100000");
    let row = ranking_row(&md, "test");
    assert!(
        row.contains("| test &amp;lt; case |"),
        "the cell carries the literal spelling with its ampersand spelled: {row}"
    );
}

#[test]
fn test_a_literal_ampersand_spelling_in_a_ranking_name_stands_with_its_ampersand_spelled() {
    let md = rendered(&records_with_name("\"test &amp; case\""), "100000");
    let row = ranking_row(&md, "test");
    assert!(
        row.contains("| test &amp;amp; case |"),
        "the cell carries the literal spelling with its ampersand spelled: {row}"
    );
}

#[test]
fn test_a_less_than_sign_in_a_pack_name_stands_as_its_entity_spelling_in_the_prose() {
    let md = rendered(&records_for_pack("\"test <fast> case\"", "\"d1\""), "100");
    let line = pack_line(&md, "fast");
    assert!(
        line.contains("- test &lt;fast> case is proposed for quarantine."),
        "the prose carries the less than sign under its entity spelling: {line}"
    );
}

#[test]
fn test_a_greater_than_sign_in_a_ranking_name_stands_as_it_stands() {
    let md = rendered(&records_with_name("\"test > case\""), "100000");
    let row = ranking_row(&md, "test");
    assert!(
        row.contains("| test > case |"),
        "the cell carries the greater than sign raw: {row}"
    );
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
