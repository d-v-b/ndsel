"""The three output-map kinds (canonical form) and their JSON encoding."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Union

from .errors import NdsqError, Reason
from .values import IndexValue, parse_index_value, require_int


@dataclass
class ConstantMap:
    offset: int


@dataclass
class SingleInputDimension:
    offset: int
    stride: int
    input_dimension: int


@dataclass
class IndexArrayMap:
    offset: int
    stride: int
    index_array: object  # raw nested JSON; deep validation deferred
    bounds: tuple[IndexValue, IndexValue]


OutputMap = Union[ConstantMap, SingleInputDimension, IndexArrayMap]


def canonicalize_output_map(raw: object) -> OutputMap:
    """Default-fill and discriminate a raw output-map object."""
    if not isinstance(raw, dict):
        raise NdsqError(Reason.INVALID_JSON, f"output map must be an object, got {raw!r}")
    offset = require_int(raw["offset"], "output.offset") if "offset" in raw else 0
    stride = require_int(raw["stride"], "output.stride") if "stride" in raw else 1
    if "index_array" in raw:
        b = raw.get("index_array_bounds", ["-inf", "+inf"])
        if not isinstance(b, list) or len(b) != 2:
            raise NdsqError(Reason.INVALID_JSON, "index_array_bounds must be a 2-element array")
        bounds = (parse_index_value(b[0]), parse_index_value(b[1]))
        return IndexArrayMap(offset=offset, stride=stride, index_array=raw["index_array"], bounds=bounds)
    if "input_dimension" in raw:
        dim = require_int(raw["input_dimension"], "output.input_dimension")
        if dim < 0:
            raise NdsqError(Reason.INVALID_JSON, f"input_dimension must be non-negative, got {dim}")
        return SingleInputDimension(offset=offset, stride=stride, input_dimension=dim)
    return ConstantMap(offset=offset)


def output_map_to_json(m: OutputMap) -> dict[str, object]:
    if isinstance(m, ConstantMap):
        return {"offset": m.offset}
    if isinstance(m, SingleInputDimension):
        return {"offset": m.offset, "stride": m.stride, "input_dimension": m.input_dimension}
    # IndexArrayMap
    return {
        "offset": m.offset,
        "stride": m.stride,
        "index_array": m.index_array,
        "index_array_bounds": [m.bounds[0], m.bounds[1]],
    }
