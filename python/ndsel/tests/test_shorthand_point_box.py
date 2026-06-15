from ndsel import NormalizedTransform, normalize, parse
from ndsel.builders import Box


def norm(text: str) -> NormalizedTransform:
    return normalize(parse(text)).to_dict()


def test_point_desugars_to_constant_maps():
    d = norm('{"kind": "point", "coords": [4, 7]}')
    assert d["input_rank"] == 0
    assert d["input_inclusive_min"] == []
    assert d["input_exclusive_max"] == []
    assert d["output"] == [{"offset": 4}, {"offset": 7}]


def test_box_desugars_to_identity():
    d = norm('{"kind": "box", "inclusive_min": [0, 0], "exclusive_max": [3, 4]}')
    assert d["output"] == [
        {"offset": 0, "stride": 1, "input_dimension": 0},
        {"offset": 0, "stride": 1, "input_dimension": 1},
    ]


def test_box_shape_only_defaults_origin_zero():
    d = norm('{"kind": "box", "shape": [5]}')
    assert d["input_inclusive_min"] == [0]
    assert d["input_exclusive_max"] == [5]


def test_normalize_accepts_a_builder():
    d = normalize(Box(shape=[5])).to_dict()
    assert d["input_exclusive_max"] == [5]
