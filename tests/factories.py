"""Synthetic record builders shared by the suitediet tests."""

from __future__ import annotations

import json


def record(name, runtime_ms, caught):
    return {"name": name, "runtime_ms": runtime_ms, "caught": list(caught)}


def records_text(tests, suite="synthetic-suite", format_name="suitediet-records/1"):
    document = {"format": format_name, "suite": suite, "tests": list(tests)}
    return json.dumps(document, indent=2, sort_keys=True) + "\n"


def example_tests():
    return [
        record("test_alpha", 120, ["d1", "d2", "d3"]),
        record("test_beta", 300, ["d2", "d3"]),
        record("test_delta", 500, ["d5"]),
        record("test_epsilon", 260, ["d6"]),
        record("test_eta", 90, []),
        record("test_gamma", 45, ["d4"]),
        record("test_zeta", 240, ["d6"]),
    ]
