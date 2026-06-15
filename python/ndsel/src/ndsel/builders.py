"""Ergonomic dataclass builders for the four shorthand message kinds."""

from __future__ import annotations

from dataclasses import dataclass

from .messages import BoxMessage, PointMessage, PointsMessage, SliceMessage


@dataclass
class Point:
    coords: list[int]

    def to_message(self) -> PointMessage:
        return {"kind": "point", "coords": self.coords}


@dataclass
class Box:
    inclusive_min: list | None = None
    exclusive_max: list | None = None
    inclusive_max: list | None = None
    shape: list | None = None
    labels: list[str] | None = None

    def to_message(self) -> BoxMessage:
        msg: dict = {"kind": "box"}
        for field in ("inclusive_min", "exclusive_max", "inclusive_max", "shape", "labels"):
            value = getattr(self, field)
            if value is not None:
                msg[field] = value
        return msg  # type: ignore[return-value]


@dataclass
class Slice:
    start: list[int]
    stop: list[int]
    step: list[int] | None = None
    labels: list[str] | None = None

    def to_message(self) -> SliceMessage:
        msg: dict = {"kind": "slice", "start": self.start, "stop": self.stop}
        if self.step is not None:
            msg["step"] = self.step
        if self.labels is not None:
            msg["labels"] = self.labels
        return msg  # type: ignore[return-value]


@dataclass
class Points:
    coords: list[list[int]]

    def to_message(self) -> PointsMessage:
        return {"kind": "points", "coords": self.coords}
