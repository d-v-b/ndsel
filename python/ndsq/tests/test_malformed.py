"""Structurally-malformed message bodies must raise invalid_json (Rust parity)."""

import pytest

from ndsq import normalize, parse
from ndsq.errors import NdsqError, Reason


def reject(text: str) -> Reason:
    with pytest.raises(NdsqError) as e:
        normalize(parse(text))
    return e.value.reason


def test_missing_required_fields_are_invalid_json():
    assert reject('{"kind": "point"}') is Reason.INVALID_JSON
    assert reject('{"kind": "slice", "start": [0]}') is Reason.INVALID_JSON
    assert reject('{"kind": "points"}') is Reason.INVALID_JSON


def test_bool_scalars_are_invalid_json():
    assert reject('{"kind": "point", "coords": [true]}') is Reason.INVALID_JSON
    assert reject('{"kind": "slice", "start": [true], "stop": [4]}') is Reason.INVALID_JSON
    assert reject('{"kind": "points", "coords": [[true]]}') is Reason.INVALID_JSON


def test_wrong_shapes_are_invalid_json():
    assert reject('{"kind": "box", "inclusive_min": 5}') is Reason.INVALID_JSON
    assert reject('{"kind": "transform", "output": 5}') is Reason.INVALID_JSON
    assert reject('{"kind": "transform", "input_labels": [1]}') is Reason.INVALID_JSON


def test_negative_rank_and_input_dimension_are_invalid_json():
    assert reject('{"kind": "transform", "input_rank": -1}') is Reason.INVALID_JSON
    assert (
        reject('{"kind": "transform", "input_shape": [2], "output": [{"input_dimension": -1}]}')
        is Reason.INVALID_JSON
    )
