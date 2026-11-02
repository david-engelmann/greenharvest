from fractions import Fraction

from suitediet import rank_tests
from suitediet.ingest import read_records_text
from suitediet.ranking import render_value

from factories import record, records_text


def ranked_by_name(tests):
    record_set = read_records_text(records_text(tests))
    return {entry.name: entry for entry in rank_tests(record_set)}


def test_a_defect_counts_in_full_to_its_only_catcher():
    entries = ranked_by_name([
        record("test_alpha", 120, ["d1"]),
        record("test_beta", 300, ["d2"]),
    ])
    assert entries["test_alpha"].weighted == Fraction(1)
    assert entries["test_alpha"].unique_catches == ("d1",)


def test_a_shared_defect_counts_divided_among_its_catchers():
    entries = ranked_by_name([
        record("test_alpha", 120, ["d1", "d2"]),
        record("test_beta", 300, ["d1"]),
        record("test_gamma", 45, ["d1"]),
    ])
    assert entries["test_alpha"].weighted == Fraction(1, 3) + 1
    assert entries["test_beta"].weighted == Fraction(1, 3)
    assert entries["test_beta"].unique_catches == ()


def test_value_is_weighted_catches_per_millisecond():
    entries = ranked_by_name([
        record("test_alpha", 120, ["d1", "d2"]),
        record("test_beta", 300, ["d2"]),
    ])
    assert entries["test_alpha"].weighted == Fraction(3, 2)
    assert entries["test_alpha"].value == Fraction(3, 2) / 120
    assert entries["test_beta"].value == Fraction(1, 2) / 300


def test_ranking_follows_the_stated_tie_rule():
    record_set = read_records_text(records_text([
        record("test_low_value", 900, ["d_low"]),
        record("test_equal_weighted_heavy", 200, ["d_a", "d_b"]),
        record("test_equal_weighted_light", 100, ["d_c"]),
        record("test_idle_slow", 800, []),
        record("test_idle_fast", 50, []),
    ]))
    order = [entry.name for entry in rank_tests(record_set)]
    # test_equal_weighted_heavy holds value 2/200 and test_equal_weighted_light
    # holds 1/100, the same value, so the heavier weighted catches rank first.
    # The two idle tests hold value zero and order by runtime ascending.
    assert order == [
        "test_equal_weighted_heavy",
        "test_equal_weighted_light",
        "test_low_value",
        "test_idle_fast",
        "test_idle_slow",
    ]


def test_identical_records_rank_by_name():
    record_set = read_records_text(records_text([
        record("test_beta", 120, ["d1"]),
        record("test_alpha", 120, ["d1"]),
    ]))
    order = [entry.name for entry in rank_tests(record_set)]
    assert order == ["test_alpha", "test_beta"]


def test_ranking_uses_exact_values_not_the_rendered_ones():
    record_set = read_records_text(records_text([
        record("test_a", 3000, ["d3"]),
        record("test_x1", 9000, ["d3"]),
        record("test_x2", 9000, ["d3"]),
        record("test_b", 4504, ["d2"]),
        record("test_y1", 9000, ["d2"]),
    ]))
    ranked = rank_tests(record_set)
    order = [entry.name for entry in ranked]
    by_name = {entry.name: entry for entry in ranked}
    # test_a holds weighted catches of one third over 3000 milliseconds and
    # test_b holds one half over 4504, so the exact values differ while the
    # rendered values agree. A ranking over the rendered values would tie
    # and fall to the weighted catches, putting test_b first.
    assert by_name["test_a"].value == Fraction(1, 9000)
    assert by_name["test_b"].value == Fraction(1, 9008)
    assert render_value(by_name["test_a"].value) == "0.000111"
    assert render_value(by_name["test_b"].value) == "0.000111"
    assert order.index("test_a") < order.index("test_b")
