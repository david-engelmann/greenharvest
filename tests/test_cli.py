from pathlib import Path

from suitediet.cli import main

ROOT = Path(__file__).resolve().parents[1]
EXAMPLES = ROOT / "examples"
EXAMPLE_BUDGET = "600"


def run_plan(records_path, out_dir):
    return main([
        "plan", str(records_path),
        "--budget-ms", EXAMPLE_BUDGET,
        "--out-dir", str(out_dir),
    ])


def test_cli_writes_deterministic_outputs(tmp_path):
    first = tmp_path / "first"
    second = tmp_path / "second"
    assert run_plan(EXAMPLES / "records.json", first) == 0
    assert run_plan(EXAMPLES / "records.json", second) == 0
    for name in ("report.json", "report.md", "quarantine.json", "remaining.json"):
        assert (first / name).read_bytes() == (second / name).read_bytes()


def test_missing_inputs_are_refused(tmp_path):
    out_dir = tmp_path / "out"
    assert main([
        "plan", str(tmp_path / "absent.json"),
        "--budget-ms", EXAMPLE_BUDGET,
        "--out-dir", str(out_dir),
    ]) == 2
    assert not (out_dir / "report.json").exists()


def test_outputs_carry_no_local_paths(tmp_path):
    out_dir = tmp_path / "out"
    assert run_plan(EXAMPLES / "records.json", out_dir) == 0
    for name in ("report.json", "report.md", "quarantine.json", "remaining.json"):
        assert str(tmp_path) not in (out_dir / name).read_text(encoding="utf-8")


def test_committed_examples_regenerate_byte_identical(tmp_path):
    out_dir = tmp_path / "out"
    assert run_plan(EXAMPLES / "records.json", out_dir) == 0
    for name in ("report.json", "report.md", "quarantine.json", "remaining.json"):
        assert (out_dir / name).read_bytes() == (EXAMPLES / name).read_bytes()
    restored = tmp_path / "restored.json"
    assert main([
        "restore", str(EXAMPLES / "remaining.json"), str(EXAMPLES / "quarantine.json"),
        "--out", str(restored),
    ]) == 0
    assert restored.read_bytes() == (EXAMPLES / "records.json").read_bytes()
