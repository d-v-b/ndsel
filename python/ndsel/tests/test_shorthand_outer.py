import pytest

from ndsel import NormalizedTransform, Outer, normalize, parse
from ndsel.errors import NdselError, Reason


def norm(text: str) -> NormalizedTransform:
    return normalize(parse(text)).to_json()


def err(text: str) -> Reason:
    with pytest.raises(NdselError) as e:
        normalize(parse(text))
    return e.value.reason


IA = {"offset": 0, "stride": 1, "index_array_bounds": ["-inf", "+inf"]}


@pytest.mark.parametrize(
    ("axes", "inclusive_min", "exclusive_max", "output"),
    [
        # The whole axis: implicit infinite bounds, identity map.
        ("[null]", [["-inf"]], [["+inf"]], [{"offset": 0, "stride": 1, "input_dimension": 0}]),
        # A range is one dimension of a `slice`: x[5:10:2] keeps its source frame.
        ('[{"start": 5, "stop": 10, "step": 2}]', [2], [5], [{"offset": 1, "stride": 2, "input_dimension": 0}]),
        # Intervals expand in order; the result is numbered [0, m).
        ('[{"intervals": [[0, 2], [5, 7]]}]', [0], [4], [{**IA, "index_array": [0, 1, 5, 6]}]),
        # The array broadcasts: m along its own dimension, 1 along every other.
        (
            '[null, {"indices": [4, 2]}, null]',
            [["-inf"], 0, ["-inf"]],
            [["+inf"], 2, ["+inf"]],
            [
                {"offset": 0, "stride": 1, "input_dimension": 0},
                {**IA, "index_array": [[[4], [2]]]},
                {"offset": 0, "stride": 1, "input_dimension": 2},
            ],
        ),
        # One coordinate collapses to a constant map; none collapses to offset 0.
        ('[{"indices": [7]}]', [0], [1], [{"offset": 7}]),
        ('[{"intervals": [[3, 3]]}]', [0], [0], [{"offset": 0}]),
    ],
)
def test_outer_desugaring(
    axes: str, inclusive_min: list, exclusive_max: list, output: list
) -> None:
    d = norm(f'{{"kind": "outer", "axes": {axes}}}')
    assert d["input_inclusive_min"] == inclusive_min
    assert d["input_exclusive_max"] == exclusive_max
    assert d["output"] == output


def test_outer_keeps_labels_on_every_form():
    d = norm('{"kind": "outer", "axes": [null, {"indices": [1, 2]}], "labels": ["y", "x"]}')
    assert d["input_labels"] == ["y", "x"]


def test_outer_builder_matches_parse():
    built = Outer(axes=[None, {"intervals": [[0, 3]]}], labels=["y", "x"])
    assert normalize(built).to_json() == norm(
        '{"kind": "outer", "axes": [null, {"intervals": [[0, 3]]}], "labels": ["y", "x"]}'
    )


def test_selector_with_members_of_two_forms_is_unknown_field():
    assert err('{"kind": "outer", "axes": [{"indices": [1], "intervals": [[0, 1]]}]}') is Reason.UNKNOWN_FIELD


def test_range_missing_stop_is_invalid_json():
    assert err('{"kind": "outer", "axes": [{"start": 0}]}') is Reason.INVALID_JSON


def test_interval_that_is_not_a_pair_is_invalid_json():
    assert err('{"kind": "outer", "axes": [{"intervals": [[1, 2, 3]]}]}') is Reason.INVALID_JSON


def test_reversed_interval_is_bounds_out_of_order():
    assert err('{"kind": "outer", "axes": [{"intervals": [[5, 3]]}]}') is Reason.BOUNDS_OUT_OF_ORDER


def test_range_step_zero():
    assert err('{"kind": "outer", "axes": [{"start": 0, "stop": 4, "step": 0}]}') is Reason.STEP_ZERO


def test_labels_length_mismatch_is_rank_mismatch():
    assert err('{"kind": "outer", "axes": [null, null], "labels": ["x"]}') is Reason.RANK_MISMATCH
