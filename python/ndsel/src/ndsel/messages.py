"""TypedDict models of the JSON message shapes (the on-the-wire layer)."""

from __future__ import annotations

from typing import Literal, NotRequired, TypedDict

# A JSON-level bound: an int, a sentinel string, or one of those wrapped in a
# one-element list (implicit). Typed loosely; semantic validation is in values.py.
BoundJson = int | str | list[int | str]


class PointMessage(TypedDict):
    kind: Literal["point"]
    coords: list[int]


class BoxMessage(TypedDict):
    kind: Literal["box"]
    inclusive_min: NotRequired[list[BoundJson]]
    exclusive_max: NotRequired[list[BoundJson]]
    inclusive_max: NotRequired[list[BoundJson]]
    shape: NotRequired[list[BoundJson]]
    labels: NotRequired[list[str]]


class SliceMessage(TypedDict):
    kind: Literal["slice"]
    start: list[int]
    stop: list[int]
    step: NotRequired[list[int]]
    labels: NotRequired[list[str]]


class PointsMessage(TypedDict):
    kind: Literal["points"]
    coords: list[list[int]]


class OutputMapDict(TypedDict):
    offset: NotRequired[int]
    stride: NotRequired[int]
    input_dimension: NotRequired[int]
    index_array: NotRequired[object]
    index_array_bounds: NotRequired[list[int | str]]


class TransformMessage(TypedDict):
    kind: Literal["transform"]
    input_rank: NotRequired[int]
    input_inclusive_min: NotRequired[list[BoundJson]]
    input_exclusive_max: NotRequired[list[BoundJson]]
    input_inclusive_max: NotRequired[list[BoundJson]]
    input_shape: NotRequired[list[BoundJson]]
    input_labels: NotRequired[list[str]]
    output: NotRequired[list[OutputMapDict]]


Message = PointMessage | BoxMessage | SliceMessage | PointsMessage | TransformMessage


# --- Normalized (output) shapes ---------------------------------------------
# Unlike the input messages above (loose, with NotRequired fields and a `kind`
# discriminator), these describe the *canonical* body that `normalize` emits:
# every field is present and there is no `kind` (spec §4.3).


class DomainFields(TypedDict):
    """The four canonical input-domain fields present in every normalized body."""

    input_rank: int
    input_inclusive_min: list[BoundJson]
    input_exclusive_max: list[BoundJson]
    input_labels: list[str]


class NormalizedTransform(DomainFields):
    """The bare canonical transform body emitted by ``Transform.to_dict`` (no ``kind``)."""

    output: list[OutputMapDict]
