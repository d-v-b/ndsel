from ndsel.output import (
    ConstantMap,
    IndexArrayMap,
    SingleInputDimension,
    canonicalize_output_map,
)


def test_constant_map_carries_only_offset():
    m = canonicalize_output_map({"offset": 3})
    assert m == ConstantMap(offset=3)
    assert m.to_json() == {"offset": 3}


def test_single_input_dimension_fills_defaults():
    m = canonicalize_output_map({"input_dimension": 2})
    assert m == SingleInputDimension(offset=0, stride=1, input_dimension=2)
    assert m.to_json() == {"offset": 0, "stride": 1, "input_dimension": 2}


def test_index_array_fills_offset_stride_bounds():
    m = canonicalize_output_map({"index_array": [1, 2, 3]})
    assert m == IndexArrayMap(offset=0, stride=1, index_array=[1, 2, 3], bounds=("-inf", "+inf"))
    assert m.to_json() == {
        "offset": 0,
        "stride": 1,
        "index_array": [1, 2, 3],
        "index_array_bounds": ["-inf", "+inf"],
    }
