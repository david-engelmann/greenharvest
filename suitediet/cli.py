"""The suitediet command line.

The plan command reads one records document and writes the diet
report as JSON and as markdown, the quarantine record, and the record
set that remains. The restore command reads the remaining record set
and a quarantine record and writes the suite record set as it stood
before the plan. A refused input writes nothing and exits nonzero.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

from .errors import SuitedietError
from .ingest import (
    quarantine_document,
    read_quarantine,
    read_records,
    read_remaining,
    records_document,
)
from .plan import build_plan
from .quarantine import build_quarantine, remaining_records, restore_records
from .report import markdown_text, report_document
from .serialize import canonical_text


def _write(path: Path, text: str) -> None:
    path.write_text(text, encoding="utf-8")


_BUDGET_PATTERN = re.compile(r"[+-]?[0-9]+")


def _parse_budget(raw: str) -> int:
    if not _BUDGET_PATTERN.fullmatch(raw.strip()):
        raise SuitedietError(f"the budget {raw!r} is not a whole number of milliseconds")
    return int(raw)


def _plan_command(args: argparse.Namespace) -> int:
    record_set = read_records(args.records)
    plan = build_plan(record_set, _parse_budget(args.budget_ms))
    quarantine = build_quarantine(record_set, plan)
    remaining = remaining_records(record_set, plan)
    report = report_document(plan)
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    _write(out_dir / "report.json", canonical_text(report))
    _write(out_dir / "report.md", markdown_text(report))
    _write(out_dir / "quarantine.json", canonical_text(quarantine_document(quarantine)))
    _write(out_dir / "remaining.json", canonical_text(records_document(remaining)))
    return 0


def _restore_command(args: argparse.Namespace) -> int:
    remaining = read_remaining(args.remaining)
    quarantine = read_quarantine(args.quarantine)
    restored = restore_records(remaining, quarantine)
    _write(Path(args.out), canonical_text(records_document(restored)))
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="suitediet")
    subcommands = parser.add_subparsers(dest="command", required=True)

    plan_parser = subcommands.add_parser("plan")
    plan_parser.add_argument("records")
    plan_parser.add_argument("--budget-ms", required=True)
    plan_parser.add_argument("--out-dir", required=True)
    plan_parser.set_defaults(handler=_plan_command)

    restore_parser = subcommands.add_parser("restore")
    restore_parser.add_argument("remaining")
    restore_parser.add_argument("quarantine")
    restore_parser.add_argument("--out", required=True)
    restore_parser.set_defaults(handler=_restore_command)

    args = parser.parse_args(argv)
    try:
        return args.handler(args)
    except SuitedietError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
