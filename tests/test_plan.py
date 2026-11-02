import pytest

from suitediet import build_plan
from suitediet.errors import SuitedietError
from suitediet.ingest import read_records_text

from factories import example_tests, record, records_text


def plan_for(tests, budget_ms):
    return build_plan(read_records_text(records_text(tests)), budget_ms)


def pack_by_name(plan):
    return {pack.ranked.name: pack for pack in plan.packs}


def test_the_cut_line_keeps_the_longest_prefix_that_fits_the_budget():
    tests = [
        record("test_first", 100, ["d1"]),
        record("test_second", 200, ["d2"]),
        record("test_third", 300, ["d3"]),
    ]
    plan = plan_for(tests, 300)
    assert [entry.name for entry in plan.survivors] == ["test_first", "test_second"]
    assert plan.survivor_runtime_ms == 300
    assert [entry.name for entry in plan.below_the_line] == ["test_third"]
    tighter = plan_for(tests, 299)
    assert [entry.name for entry in tighter.survivors] == ["test_first"]
    assert [entry.name for entry in tighter.below_the_line] == ["test_second", "test_third"]


def test_a_budget_below_the_best_test_cuts_every_test():
    plan = plan_for([record("test_alpha", 500, ["d1"])], 499)
    assert plan.survivors == ()
    assert [entry.name for entry in plan.below_the_line] == ["test_alpha"]


def test_a_budget_covering_the_whole_suite_cuts_nothing():
    tests = [record("test_alpha", 100, ["d1"]), record("test_beta", 200, ["d2"])]
    for budget in (300, 10_000):
        plan = plan_for(tests, budget)
        assert plan.below_the_line == ()
        assert plan.packs == ()


def test_a_non_positive_budget_is_refused():
    record_set = read_records_text(records_text([record("test_alpha", 100, ["d1"])]))
    for budget in (0, -1, 1.5, True):
        with pytest.raises(SuitedietError):
            build_plan(record_set, budget)


def test_a_unique_catch_blocks_removal_even_at_the_lowest_value():
    tests = [
        record("test_fast", 50, ["d1"]),
        record("test_unique", 5000, ["d_unique"]),
    ]
    plan = plan_for(tests, 50)
    packs = pack_by_name(plan)
    unique = packs["test_unique"]
    assert unique.ranked.value == min(entry.value for entry in plan.ranking)
    assert unique.disposition == "blocked"
    assert unique.blocking_defects == ("d_unique",)
    assert "test_unique" in plan.remaining_names
    example = plan_for(example_tests(), 600)
    delta = pack_by_name(example)["test_delta"]
    assert delta.disposition == "blocked"
    assert delta.blocking_defects == ("d5",)
    assert "test_delta" in example.remaining_names


def test_a_test_with_no_unique_catches_is_clear_to_propose():
    plan = plan_for(example_tests(), 600)
    packs = pack_by_name(plan)
    assert packs["test_epsilon"].disposition == "quarantine"
    assert packs["test_epsilon"].blocking_defects == ()
    assert "test_epsilon" not in plan.remaining_names


def test_a_test_that_catches_nothing_is_clear_when_below_the_line():
    plan = plan_for(example_tests(), 600)
    packs = pack_by_name(plan)
    assert packs["test_eta"].disposition == "quarantine"
    assert packs["test_eta"].ranked.record.caught == ()


def test_two_tests_that_only_cover_each_other_split_clear_and_blocked():
    tests = [
        record("test_top", 100, ["d_top"]),
        record("test_better", 200, ["dx"]),
        record("test_worse", 300, ["dx"]),
    ]
    plan = plan_for(tests, 100)
    packs = pack_by_name(plan)
    assert packs["test_worse"].disposition == "quarantine"
    assert packs["test_better"].disposition == "blocked"
    assert packs["test_better"].blocking_defects == ("dx",)
    assert plan.defects_after == plan.defects_before
    assert plan.lost_defects == ()


def test_a_removal_pack_carries_the_record_and_the_exact_overlap():
    plan = plan_for(example_tests(), 600)
    packs = pack_by_name(plan)
    epsilon = packs["test_epsilon"]
    assert epsilon.ranked.record.name == "test_epsilon"
    assert epsilon.ranked.record.runtime_ms == 260
    assert epsilon.ranked.record.caught == ("d6",)
    assert dict(epsilon.overlap) == {"d6": ("test_zeta",)}
    eta = packs["test_eta"]
    assert dict(eta.overlap) == {}
    # Packs are listed in the order the decisions were taken, worst first.
    assert [pack.ranked.name for pack in plan.packs] == [
        "test_eta", "test_epsilon", "test_delta", "test_zeta",
    ]


def test_a_blocked_pack_names_every_blocking_defect():
    tests = [
        record("test_top", 100, ["d_top"]),
        record("test_keeper", 900, ["d_a", "d_b"]),
    ]
    plan = plan_for(tests, 100)
    packs = pack_by_name(plan)
    keeper = packs["test_keeper"]
    assert keeper.disposition == "blocked"
    assert keeper.blocking_defects == ("d_a", "d_b")
    assert dict(keeper.overlap) == {"d_a": (), "d_b": ()}


def test_totals_count_the_catches_before_and_after():
    plan = plan_for(example_tests(), 600)
    assert plan.defects_before == ("d1", "d2", "d3", "d4", "d5", "d6")
    assert plan.defects_after == ("d1", "d2", "d3", "d4", "d5", "d6")
    assert plan.lost_defects == ()


def test_the_after_total_never_claims_a_catch_the_plan_loses():
    record_set = read_records_text(records_text(example_tests()))
    plan = build_plan(record_set, 600)
    by_name = record_set.by_name()
    recomputed_after = tuple(sorted(
        {defect for name in plan.remaining_names for defect in by_name[name].caught}
    ))
    assert plan.defects_after == recomputed_after
    assert plan.lost_defects == tuple(
        defect for defect in plan.defects_before if defect not in set(recomputed_after)
    )
    # A shared defect whose one catcher is quarantined still counts after,
    # because its other catcher stands. d6 is caught after by test_zeta.
    assert "d6" in plan.defects_after
    assert "test_epsilon" not in plan.remaining_names
