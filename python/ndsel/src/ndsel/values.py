"""Index values (finite or +/-inf) and bounds with the [n]-bracket implicit convention."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

from .errors import NdselError, Reason
from .messages import BoundJson

# A single index coordinate: a finite int, or the infinity sentinels.
IndexValue = int | Literal["-inf", "+inf"]

# Canonical integer range: 64-bit signed (spec §3.5).
I64_MIN = -(2**63)
I64_MAX = 2**63 - 1


def parse_index_value(raw: object) -> IndexValue:
    """Validate a JSON scalar as an IndexValue. Rejects bool (a Python int subclass)."""
    if isinstance(raw, bool):
        raise NdselError(Reason.INVALID_JSON, f"index value must be an integer, got bool {raw!r}")
    if isinstance(raw, int):
        if not (I64_MIN <= raw <= I64_MAX):
            raise NdselError(Reason.INVALID_JSON, f"index value {raw} is out of 64-bit signed range")
        return raw
    if raw == "-inf" or raw == "+inf":
        return raw  # type: ignore[return-value]
    raise NdselError(Reason.INVALID_JSON, f"invalid index value: {raw!r}")


def require_int(raw: object, what: str) -> int:
    """Validate that a JSON value is a plain integer in 64-bit signed range (rejecting bool)."""
    if isinstance(raw, bool) or not isinstance(raw, int):
        raise NdselError(Reason.INVALID_JSON, f"{what} must be an integer, got {raw!r}")
    if not (I64_MIN <= raw <= I64_MAX):
        raise NdselError(Reason.INVALID_JSON, f"{what} value {raw} is out of 64-bit signed range")
    return raw


def require_int_list(raw: object, what: str) -> list[int]:
    """Validate that a JSON value is an array of plain integers."""
    if not isinstance(raw, list):
        raise NdselError(Reason.INVALID_JSON, f"{what} must be an array of integers, got {raw!r}")
    return [require_int(v, f"{what}[{i}]") for i, v in enumerate(raw)]


def require_list(raw: object, what: str) -> list[object]:
    """Validate that a JSON value is an array (element types checked by the caller)."""
    if not isinstance(raw, list):
        raise NdselError(Reason.INVALID_JSON, f"{what} must be an array, got {raw!r}")
    return raw


def require_str_list(raw: object, what: str) -> list[str]:
    """Validate that a JSON value is an array of strings."""
    result: list[str] = []
    for i, v in enumerate(require_list(raw, what)):
        if not isinstance(v, str):
            raise NdselError(Reason.INVALID_JSON, f"{what}[{i}] must be a string, got {v!r}")
        result.append(v)
    return result


@dataclass(frozen=True, slots=True, kw_only=True)
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
                raise NdselError(Reason.INVALID_JSON, "implicit bound must be a 1-element array")
            return cls(value=parse_index_value(raw[0]), implicit=True)
        return cls(value=parse_index_value(raw), implicit=False)

    def to_json(self) -> BoundJson:
        return [self.value] if self.implicit else self.value
