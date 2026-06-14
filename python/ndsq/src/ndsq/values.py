"""Index values (finite or +/-inf) and bounds with the [n]-bracket implicit convention."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal, Union

from .errors import NdsqError, Reason

# A single index coordinate: a finite int, or the infinity sentinels.
IndexValue = Union[int, Literal["-inf", "+inf"]]


def parse_index_value(raw: object) -> IndexValue:
    """Validate a JSON scalar as an IndexValue. Rejects bool (a Python int subclass)."""
    if isinstance(raw, bool):
        raise NdsqError(Reason.INVALID_JSON, f"index value must be an integer, got bool {raw!r}")
    if isinstance(raw, int):
        return raw
    if raw == "-inf" or raw == "+inf":
        return raw  # type: ignore[return-value]
    raise NdsqError(Reason.INVALID_JSON, f"invalid index value: {raw!r}")


@dataclass(frozen=True)
class ImplicitValue:
    """An index bound plus an explicit/implicit flag.

    JSON: a bare value is explicit (`7`, `"-inf"`); the same wrapped in a
    one-element array is implicit (`[7]`, `["-inf"]`).
    """

    value: IndexValue
    implicit: bool

    @classmethod
    def explicit(cls, value: IndexValue) -> "ImplicitValue":
        return cls(value=value, implicit=False)

    @classmethod
    def from_json(cls, raw: object) -> "ImplicitValue":
        if isinstance(raw, list):
            if len(raw) != 1:
                raise NdsqError(Reason.INVALID_JSON, "implicit bound must be a 1-element array")
            return cls(value=parse_index_value(raw[0]), implicit=True)
        return cls(value=parse_index_value(raw), implicit=False)

    def to_json(self) -> object:
        return [self.value] if self.implicit else self.value
