"""The three output-map kinds (canonical form) and their JSON encoding."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Union

from .values import IndexValue, parse_index_value


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
    index_array: Any  # raw nested JSON; deep validation deferred
    bounds: tuple[IndexValue, IndexValue]


OutputMap = Union[ConstantMap, SingleInputDimension, IndexArrayMap]


def canonicalize_output_map(raw: dict) -> OutputMap:
    """Default-fill and discriminate a raw output-map object."""
    offset = raw.get("offset", 0)
    stride = raw.get("stride", 1)
    if "index_array" in raw:
        b = raw.get("index_array_bounds", ["-inf", "+inf"])
        bounds = (parse_index_value(b[0]), parse_index_value(b[1]))
        return IndexArrayMap(offset=offset, stride=stride, index_array=raw["index_array"], bounds=bounds)
    if "input_dimension" in raw:
        return SingleInputDimension(offset=offset, stride=stride, input_dimension=raw["input_dimension"])
    return ConstantMap(offset=offset)


def output_map_to_json(m: OutputMap) -> dict:
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
