import pytest

from ndsel import NormalizedTransform, normalize, parse
from ndsel.errors import NdselError, Reason


def norm(text: str) -> NormalizedTransform:
    return normalize(parse(text)).to_json()


def test_points_transpose_to_columnar():
    d = norm('{"kind": "points", "coords": [[1, 10], [2, 20], [3, 30]]}')
    assert d["input_rank"] == 1
    assert d["input_inclusive_min"] == [0]
    assert d["input_exclusive_max"] == [3]
    assert d["output"] == [
        {"offset": 0, "stride": 1, "index_array": [1, 2, 3], "index_array_bounds": ["-inf", "+inf"]},
        {"offset": 0, "stride": 1, "index_array": [10, 20, 30], "index_array_bounds": ["-inf", "+inf"]},
    ]


def test_empty_points():
    d = norm('{"kind": "points", "coords": []}')
    assert d["input_rank"] == 1
    assert d["input_exclusive_max"] == [0]
    assert d["output"] == []


def test_ragged_points_is_rank_mismatch():
    with pytest.raises(NdselError) as e:
        normalize(parse('{"kind": "points", "coords": [[1, 2], [3]]}'))
    assert e.value.reason is Reason.RANK_MISMATCH
