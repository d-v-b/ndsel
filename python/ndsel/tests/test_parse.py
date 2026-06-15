import pytest

from ndsel import normalize, parse
from ndsel.errors import NdselError, Reason


def test_unknown_kind():
    with pytest.raises(NdselError) as e:
        parse('{"kind": "bogus"}')
    assert e.value.reason is Reason.UNKNOWN_KIND


def test_missing_kind_is_invalid_json():
    with pytest.raises(NdselError) as e:
        parse('{"coords": [1]}')
    assert e.value.reason is Reason.INVALID_JSON


def test_malformed_json_is_invalid_json():
    with pytest.raises(NdselError) as e:
        parse("{ not json")
    assert e.value.reason is Reason.INVALID_JSON


def test_non_object_is_invalid_json():
    with pytest.raises(NdselError) as e:
        parse("[1, 2, 3]")
    assert e.value.reason is Reason.INVALID_JSON


def test_parse_then_normalize_roundtrip():
    t = normalize(parse('{"kind": "box", "shape": [2, 3]}'))
    assert t.to_dict()["input_exclusive_max"] == [2, 3]
