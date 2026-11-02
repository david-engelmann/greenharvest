import json

import pytest

from suitediet import build_plan
from suitediet.cli import main
from suitediet.errors import SuitedietError
from suitediet.ingest import read_records_text
from suitediet.report import markdown_text, report_document

from factories import example_tests, record, records_text


def test_restore_undoes_a_plan_that_cleared_every_test(tmp_path):
    idle = [
        record("test_a", 90, []),
        record("test_b", 50, []),
        record("test_c", 10, []),
    ]
    records_path = tmp_path / "records.json"
    records_path.write_text(records_text(idle), encoding="utf-8")
    out_dir = tmp_path / "out"
    assert main([
        "plan", str(records_path),
        "--budget-ms", "5",
        "--out-dir", str(out_dir),
    ]) == 0
    remaining = json.loads((out_dir / "remaining.json").read_text(encoding="utf-8"))
    assert remaining["tests"] == []
    quarantine = json.loads((out_dir / "quarantine.json").read_text(encoding="utf-8"))
    assert [test["name"] for test in quarantine["tests"]] == [
        "test_a", "test_b", "test_c",
    ]
    restored = tmp_path / "restored.json"
    assert main([
        "restore", str(out_dir / "remaining.json"), str(out_dir / "quarantine.json"),
        "--out", str(restored),
    ]) == 0
    assert restored.read_bytes() == records_path.read_bytes()


def test_a_repeated_document_key_is_refused():
    text = (
        '{"format": "suitediet-records/1", "suite": "first", "suite": "second", '
        '"tests": [{"name": "test_a", "runtime_ms": 10, "caught": []}]}'
    )
    with pytest.raises(SuitedietError):
        read_records_text(text)


def test_a_repeated_record_key_is_refused():
    text = (
        '{"format": "suitediet-records/1", "suite": "s", '
        '"tests": [{"name": "test_a", "runtime_ms": 10, "runtime_ms": 99999, '
        '"caught": []}]}'
    )
    with pytest.raises(SuitedietError):
        read_records_text(text)


def test_a_repeated_format_key_cannot_hide_a_wrong_format_name():
    text = (
        '{"format": "suitediet-records/2", "format": "suitediet-records/1", '
        '"suite": "s", '
        '"tests": [{"name": "test_a", "runtime_ms": 10, "caught": []}]}'
    )
    with pytest.raises(SuitedietError):
        read_records_text(text)


def test_a_non_whole_number_budget_is_refused_at_the_command_line(tmp_path, capsys):
    records_path = tmp_path / "records.json"
    records_path.write_text(
        records_text([record("test_a", 10, [])]), encoding="utf-8"
    )
    for budget in ("12.5", "ten"):
        out_dir = tmp_path / "out"
        assert main([
            "plan", str(records_path),
            "--budget-ms", budget,
            "--out-dir", str(out_dir),
        ]) == 2
        assert not (out_dir / "report.json").exists()
    assert "is not a whole number of milliseconds" in capsys.readouterr().err


def test_the_markdown_report_states_the_pack_overlap():
    record_set = read_records_text(records_text(example_tests()))
    text = markdown_text(report_document(build_plan(record_set, 600)))
    assert "- test_eta is proposed for quarantine." in text
    assert (
        "- test_epsilon is proposed for quarantine. "
        "Overlap with the tests still standing after the plan. "
        "d6 is also caught by test_zeta."
    ) in text
    assert (
        "- test_delta is held in the suite. Its removal would lose d5. "
        "Overlap with the tests still standing after the plan. "
        "d5 is also caught by no other test still standing."
    ) in text
    assert (
        "- test_zeta is held in the suite. Its removal would lose d6. "
        "Overlap with the tests still standing after the plan. "
        "d6 is also caught by no other test still standing."
    ) in text
    pair = read_records_text(records_text([
        record("test_keeper_a", 100, ["d1"]),
        record("test_keeper_b", 100, ["d2"]),
        record("test_weak", 900, ["d1", "d2"]),
    ]))
    pair_text = markdown_text(report_document(build_plan(pair, 200)))
    assert (
        "- test_weak is proposed for quarantine. "
        "Overlap with the tests still standing after the plan. "
        "d1 is also caught by test_keeper_a. "
        "d2 is also caught by test_keeper_b."
    ) in pair_text
    shared = read_records_text(records_text([
        record("test_keeper_a", 100, ["d1"]),
        record("test_keeper_b", 100, ["d1"]),
        record("test_weak", 900, ["d1"]),
    ]))
    shared_text = markdown_text(report_document(build_plan(shared, 200)))
    assert (
        "d1 is also caught by test_keeper_a and test_keeper_b."
    ) in shared_text
