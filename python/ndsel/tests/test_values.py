import pytest

from ndsel.errors import NdsqError, Reason
from ndsel.values import ImplicitValue, parse_index_value


def test_parse_index_value_int_and_sentinels():
    assert parse_index_value(7) == 7
    assert parse_index_value("-inf") == "-inf"
    assert parse_index_value("+inf") == "+inf"


def test_parse_index_value_rejects_bool_and_garbage():
    with pytest.raises(NdsqError) as e1:
        parse_index_value(True)
    assert e1.value.reason is Reason.INVALID_JSON
    with pytest.raises(NdsqError):
        parse_index_value("nope")


def test_bare_value_is_explicit():
    v = ImplicitValue.from_json(7)
    assert v.value == 7 and v.implicit is False
    assert v.to_json() == 7


def test_bracketed_value_is_implicit():
    v = ImplicitValue.from_json([7])
    assert v.value == 7 and v.implicit is True
    assert v.to_json() == [7]


def test_bracketed_sentinel_is_implicit():
    v = ImplicitValue.from_json(["-inf"])
    assert v.value == "-inf" and v.implicit is True
    assert v.to_json() == ["-inf"]


def test_multi_element_bracket_is_invalid():
    with pytest.raises(NdsqError) as e:
        ImplicitValue.from_json([1, 2])
    assert e.value.reason is Reason.INVALID_JSON


def test_explicit_constructor():
    assert ImplicitValue.explicit(0) == ImplicitValue(value=0, implicit=False)
