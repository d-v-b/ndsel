from ndsq.transform import Transform, canonicalize_transform


def test_omitted_output_becomes_explicit_identity():
    t = canonicalize_transform(
        {"input_inclusive_min": [0, 0], "input_exclusive_max": [3, 4]}
    )
    d = t.to_dict()
    assert d["input_inclusive_min"] == [0, 0]
    assert d["input_exclusive_max"] == [3, 4]
    assert d["output"] == [
        {"offset": 0, "stride": 1, "input_dimension": 0},
        {"offset": 0, "stride": 1, "input_dimension": 1},
    ]


def test_canonicalize_is_idempotent():
    once = canonicalize_transform({"input_inclusive_min": [0], "input_shape": [5]})
    twice = canonicalize_transform(once.to_dict())
    assert once.to_dict() == twice.to_dict()


def test_explicit_output_all_three_kinds():
    t = canonicalize_transform(
        {
            "input_inclusive_min": [0],
            "input_exclusive_max": [3],
            "output": [
                {"offset": 7},
                {"input_dimension": 0, "stride": 2},
                {"index_array": [1, 2, 3]},
            ],
        }
    )
    assert t.to_dict()["output"] == [
        {"offset": 7},
        {"offset": 0, "stride": 2, "input_dimension": 0},
        {"offset": 0, "stride": 1, "index_array": [1, 2, 3], "index_array_bounds": ["-inf", "+inf"]},
    ]


def test_isinstance():
    assert isinstance(canonicalize_transform({"input_shape": [1]}), Transform)
