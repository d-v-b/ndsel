import pytest

from ndsq.domain import Domain, canonicalize_domain
from ndsq.errors import NdsqError, Reason
from ndsq.values import ImplicitValue


def fin(n: int) -> ImplicitValue:
    return ImplicitValue.explicit(n)


def test_shape_only_defaults_min_to_zero():
    d = canonicalize_domain(shape=[3, 4])
    assert d.rank == 2
    assert d.inclusive_min == [fin(0), fin(0)]
    assert d.exclusive_max == [fin(3), fin(4)]
    assert d.labels == ["", ""]


def test_inclusive_max_converts_to_exclusive():
    d = canonicalize_domain(inclusive_min=[0], inclusive_max=[9])
    assert d.exclusive_max == [fin(10)]


def test_implicit_and_infinite_bounds_preserved():
    d = canonicalize_domain(inclusive_min=[["-inf"], 0], exclusive_max=[["+inf"], 4])
    assert d.inclusive_min == [ImplicitValue("-inf", True), fin(0)]
    assert d.exclusive_max == [ImplicitValue("+inf", True), fin(4)]


def test_multiple_upper_bounds_is_error():
    with pytest.raises(NdsqError) as e:
        canonicalize_domain(shape=[3], exclusive_max=[3])
    assert e.value.reason is Reason.MULTIPLE_UPPER_BOUNDS


def test_length_disagreement_is_rank_mismatch():
    with pytest.raises(NdsqError) as e:
        canonicalize_domain(inclusive_min=[0, 0], shape=[3])
    assert e.value.reason is Reason.RANK_MISMATCH


def test_rank_field_disagreement_is_rank_mismatch():
    with pytest.raises(NdsqError) as e:
        canonicalize_domain(rank=2, inclusive_min=[0])
    assert e.value.reason is Reason.RANK_MISMATCH


def test_domain_to_json_fields():
    d = canonicalize_domain(inclusive_min=[0, 0], exclusive_max=[3, 4])
    assert d.to_json_fields() == {
        "input_rank": 2,
        "input_inclusive_min": [0, 0],
        "input_exclusive_max": [3, 4],
        "input_labels": ["", ""],
    }
