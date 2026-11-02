import json

import pytest

from suitediet import derive_catchers
from suitediet.errors import SuitedietError
from suitediet.ingest import read_records_text

from factories import record, records_text


def test_records_document_is_read_exactly():
    text = records_text([
        record("test_beta", 300, ["d3", "d2"]),
        record("test_alpha", 120, ["d1"]),
    ])
    record_set = read_records_text(text)
    assert record_set.suite == "synthetic-suite"
    assert [test.name for test in record_set.tests] == ["test_alpha", "test_beta"]
    alpha, beta = record_set.tests
    assert (alpha.runtime_ms, alpha.caught) == (120, ("d1",))
    assert (beta.runtime_ms, beta.caught) == (300, ("d2", "d3"))


def test_records_with_a_missing_or_unexpected_field_are_refused():
    good = record("test_alpha", 120, ["d1"])
    variants = [
        {"format": "suitediet-records/1", "tests": [good]},
        {"format": "suitediet-records/1", "suite": "s", "tests": [good], "owner": "x"},
        {"format": "suitediet-records/1", "suite": "s", "tests": [
            {"name": "test_alpha", "runtime_ms": 120}]},
        {"format": "suitediet-records/1", "suite": "s", "tests": [
            {"name": "test_alpha", "runtime_ms": 120, "caught": [], "flaky": True}]},
    ]
    for variant in variants:
        with pytest.raises(SuitedietError):
            read_records_text(json.dumps(variant))


def test_wrong_format_name_is_refused():
    text = records_text([record("test_alpha", 120, ["d1"])], format_name="suitediet-records/2")
    with pytest.raises(SuitedietError):
        read_records_text(text)


def test_an_empty_tests_list_is_refused():
    with pytest.raises(SuitedietError):
        read_records_text(records_text([]))


def test_duplicate_test_names_are_refused():
    text = records_text([
        record("test_alpha", 120, ["d1"]),
        record("test_alpha", 90, ["d2"]),
    ])
    with pytest.raises(SuitedietError):
        read_records_text(text)


def test_a_duplicate_defect_within_one_test_is_refused():
    text = records_text([record("test_alpha", 120, ["d1", "d1"])])
    with pytest.raises(SuitedietError):
        read_records_text(text)


def test_a_non_positive_runtime_is_refused():
    for runtime in (0, -5, 1.5, True):
        text = records_text([record("test_alpha", runtime, ["d1"])])
        with pytest.raises(SuitedietError):
            read_records_text(text)


def test_catchers_are_derived_from_the_records_alone():
    record_set = read_records_text(records_text([
        record("test_alpha", 120, ["d1", "d2"]),
        record("test_beta", 300, ["d2", "d3"]),
        record("test_gamma", 45, []),
    ]))
    catchers = derive_catchers(record_set)
    assert catchers == {
        "d1": ("test_alpha",),
        "d2": ("test_alpha", "test_beta"),
        "d3": ("test_beta",),
    }
    total_entries = sum(len(test.caught) for test in record_set.tests)
    assert sum(len(names) for names in catchers.values()) == total_entries
