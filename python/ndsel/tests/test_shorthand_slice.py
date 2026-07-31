import pytest

from ndsel import NormalizedTransform, normalize, parse
from ndsel.errors import NdselError, Reason


def norm(text: str) -> NormalizedTransform:
    return normalize(parse(text)).to_json()


def err(text: str) -> Reason:
    with pytest.raises(NdselError) as e:
        normalize(parse(text))
    return e.value.reason


def test_unit_step_preserves_source_frame():
    d = norm('{"kind": "slice", "start": [5], "stop": [10]}')
    assert d["input_inclusive_min"] == [5]
    assert d["input_exclusive_max"] == [10]
    assert d["output"] == [{"offset": 0, "stride": 1, "input_dimension": 0}]


def test_divisible_strided_slice():
    d = norm('{"kind": "slice", "start": [4], "stop": [10], "step": [2]}')
    assert d["input_inclusive_min"] == [2]
    assert d["input_exclusive_max"] == [5]
    assert d["output"] == [{"offset": 0, "stride": 2, "input_dimension": 0}]


def test_nondivisible_strided_slice_phase_offset():
    d = norm('{"kind": "slice", "start": [5], "stop": [10], "step": [2]}')
    assert d["input_inclusive_min"] == [2]
    assert d["input_exclusive_max"] == [5]
    assert d["output"] == [{"offset": 1, "stride": 2, "input_dimension": 0}]


def test_empty_slice_zero_length():
    d = norm('{"kind": "slice", "start": [10], "stop": [10]}')
    assert d["input_inclusive_min"] == [10]
    assert d["input_exclusive_max"] == [10]


def test_2d_mixed_step():
    d = norm('{"kind": "slice", "start": [0, 5], "stop": [10, 10], "step": [2, 1]}')
    assert d["input_inclusive_min"] == [0, 5]
    assert d["input_exclusive_max"] == [5, 10]


@pytest.mark.parametrize(
    ("start", "stop", "step", "inclusive_min", "exclusive_max", "offset"),
    [
        # x[19:-1:-1] — a reversed length-20 axis: domain [-19, 1), points 19..0.
        (19, -1, -1, -19, 1, 0),
        # x[15:5:-2] — divisible span: points 15,13,11,9,7.
        (15, 5, -2, -7, -2, 1),
        # x[15:5:-4] — span 10 is not divisible by 4: points 15,11,7.
        (15, 5, -4, -3, 0, 3),
        # x[-1:-6:-2] — negative interval; trunc(-1/-2) = 0, so the origin is 0.
        (-1, -6, -2, 0, 3, -1),
        # x[5:4:-3] — one point.
        (5, 4, -3, -1, 0, 2),
        # x[5:5:-1] — empty is legal anywhere, at the origin trunc(5/-1) = -5.
        (5, 5, -1, -5, -5, 0),
    ],
)
def test_negative_step(
    start: int, stop: int, step: int, inclusive_min: int, exclusive_max: int, offset: int
) -> None:
    d = norm(f'{{"kind": "slice", "start": [{start}], "stop": [{stop}], "step": [{step}]}}')
    assert d["input_inclusive_min"] == [inclusive_min]
    assert d["input_exclusive_max"] == [exclusive_max]
    assert d["output"] == [{"offset": offset, "stride": step, "input_dimension": 0}]


def test_negative_step_keeps_labels():
    d = norm('{"kind": "slice", "start": [19], "stop": [-1], "step": [-1], "labels": ["x"]}')
    assert d["input_labels"] == ["x"]


def test_reversed_interval_positive_step_is_error():
    assert err('{"kind": "slice", "start": [9], "stop": [0], "step": [2]}') is Reason.BOUNDS_OUT_OF_ORDER


def test_reversed_interval_negative_step_is_error():
    assert err('{"kind": "slice", "start": [5], "stop": [6], "step": [-1]}') is Reason.BOUNDS_OUT_OF_ORDER


def test_errors():
    assert err('{"kind": "slice", "start": [0], "stop": [4], "step": [0]}') is Reason.STEP_ZERO
    assert err('{"kind": "slice", "start": [0, 0], "stop": [4]}') is Reason.RANK_MISMATCH
