"""Ergonomic dataclass builders for the four shorthand message kinds."""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from .messages import BoundJson, BoxMessage, PointMessage, PointsMessage, SliceMessage


@dataclass
class Point:
    coords: Sequence[int]

    def to_message(self) -> PointMessage:
        return {"kind": "point", "coords": list(self.coords)}


@dataclass
class Box:
    inclusive_min: Sequence[BoundJson] | None = None
    exclusive_max: Sequence[BoundJson] | None = None
    inclusive_max: Sequence[BoundJson] | None = None
    shape: Sequence[BoundJson] | None = None
    labels: Sequence[str] | None = None

    def to_message(self) -> BoxMessage:
        msg: dict[str, object] = {"kind": "box"}
        for field in ("inclusive_min", "exclusive_max", "inclusive_max", "shape", "labels"):
            value = getattr(self, field)
            if value is not None:
                msg[field] = list(value)
        return msg  # type: ignore[return-value]


@dataclass
class Slice:
    start: Sequence[int]
    stop: Sequence[int]
    step: Sequence[int] | None = None
    labels: Sequence[str] | None = None

    def to_message(self) -> SliceMessage:
        msg: dict[str, object] = {"kind": "slice", "start": list(self.start), "stop": list(self.stop)}
        if self.step is not None:
            msg["step"] = list(self.step)
        if self.labels is not None:
            msg["labels"] = list(self.labels)
        return msg  # type: ignore[return-value]


@dataclass
class Points:
    coords: Sequence[Sequence[int]]

    def to_message(self) -> PointsMessage:
        return {"kind": "points", "coords": [list(p) for p in self.coords]}
