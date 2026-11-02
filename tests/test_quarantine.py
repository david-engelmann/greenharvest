import pytest

from suitediet import build_plan
from suitediet.errors import SuitedietError
from suitediet.ingest import Quarantine, read_records_text, records_document
from suitediet.quarantine import build_quarantine, remaining_records, restore_records
from suitediet.serialize import canonical_text

from factories import example_tests, records_text


def example_plan():
    record_set = read_records_text(records_text(example_tests()))
    return record_set, build_plan(record_set, 600)


def test_quarantine_carries_only_the_cleared_tests_with_their_full_records():
    record_set, plan = example_plan()
    quarantine = build_quarantine(record_set, plan)
    assert [test.name for test in quarantine.tests] == ["test_epsilon", "test_eta"]
    by_name = record_set.by_name()
    for test in quarantine.tests:
        assert test == by_name[test.name]


def test_a_blocked_test_never_enters_quarantine():
    record_set, plan = example_plan()
    quarantine = build_quarantine(record_set, plan)
    names = {test.name for test in quarantine.tests}
    assert "test_delta" not in names
    assert "test_zeta" not in names
    remaining = remaining_records(record_set, plan)
    assert {test.name for test in remaining.tests} == set(plan.remaining_names)


def test_restore_returns_the_suite_record_exactly():
    record_set, plan = example_plan()
    quarantine = build_quarantine(record_set, plan)
    remaining = remaining_records(record_set, plan)
    restored = restore_records(remaining, quarantine)
    assert canonical_text(records_document(restored)) == canonical_text(
        records_document(record_set)
    )


def test_restore_refuses_a_quarantine_that_disagrees():
    record_set, plan = example_plan()
    quarantine = build_quarantine(record_set, plan)
    remaining = remaining_records(record_set, plan)
    wrong_suite = Quarantine(suite="another-suite", tests=quarantine.tests)
    with pytest.raises(SuitedietError):
        restore_records(remaining, wrong_suite)
    standing = record_set.by_name()["test_alpha"]
    colliding = Quarantine(suite=record_set.suite, tests=(standing,))
    with pytest.raises(SuitedietError):
        restore_records(remaining, colliding)
