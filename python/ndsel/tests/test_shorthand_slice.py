import pytest

from ndsel import NormalizedTransform, normalize, parse
from ndsel.errors import NdsqError, Reason


def norm(text: str) -> NormalizedTransform:
    return normalize(parse(text)).to_dict()


def err(text: str) -> Reason:
    with pytest.raises(NdsqError) as e:
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


def test_errors():
    assert err('{"kind": "slice", "start": [0], "stop": [4], "step": [0]}') is Reason.STEP_ZERO
    assert err('{"kind": "slice", "start": [9], "stop": [0], "step": [-2]}') is Reason.NEGATIVE_STEP_UNSUPPORTED
    assert err('{"kind": "slice", "start": [0, 0], "stop": [4]}') is Reason.RANK_MISMATCH
