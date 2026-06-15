"""ndsel — JSON-serializable n-dimensional spatial queries.

Parse a JSON string with `parse`, then reduce it to a canonical `Transform`
with `normalize`. `normalize` also accepts a builder dataclass directly.
"""

from __future__ import annotations

import json

from .builders import Box, Point, Points, Slice
from .domain import Domain
from .errors import NdselError, Reason
from .messages import Message, NormalizedTransform
from .output import ConstantMap, IndexArrayMap, OutputMap, SingleInputDimension
from .shorthand import desugar_box, desugar_point, desugar_points, desugar_slice
from .transform import Transform, canonicalize_transform
from .values import MaybeImplicitValue, IndexValue

__all__ = [
    "parse",
    "normalize",
    "Transform",
    "Domain",
    "OutputMap",
    "ConstantMap",
    "SingleInputDimension",
    "IndexArrayMap",
    "MaybeImplicitValue",
    "IndexValue",
    "Message",
    "NormalizedTransform",
    "Point",
    "Box",
    "Slice",
    "Points",
    "NdselError",
    "Reason",
]

_KNOWN_KINDS = {"point", "box", "slice", "points", "transform"}
_BUILDERS = (Point, Box, Slice, Points)


def parse(text: str) -> Message:
    """Parse a JSON string and validate the `kind` discriminator.

    Raises ``NdselError(invalid_json)`` for non-JSON, non-object, or missing-`kind`
    input, and ``NdselError(unknown_kind)`` for an unrecognized kind. Deeper
    structural validation of the message body (field types, required fields)
    happens in `normalize`.
    """
    try:
        obj = json.loads(text)
    except (json.JSONDecodeError, TypeError) as exc:
        raise NdselError(Reason.INVALID_JSON, str(exc)) from exc
    if not isinstance(obj, dict):
        raise NdselError(Reason.INVALID_JSON, "message must be a JSON object")
    kind = obj.get("kind")
    if not isinstance(kind, str):
        raise NdselError(Reason.INVALID_JSON, "missing string `kind`")
    if kind not in _KNOWN_KINDS:
        raise NdselError(Reason.UNKNOWN_KIND, f"unknown kind: {kind}")
    return obj  # type: ignore[return-value]


def normalize(message: Message | Point | Box | Slice | Points) -> Transform:
    """Reduce a message (or a builder dataclass) to its canonical Transform."""
    if isinstance(message, _BUILDERS):
        message = message.to_json()
    # Discriminate on `kind` inline so each branch narrows `message` to the
    # concrete message TypedDict the desugarer expects.
    if message["kind"] == "point":
        return desugar_point(message)
    if message["kind"] == "box":
        return desugar_box(message)
    if message["kind"] == "slice":
        return desugar_slice(message)
    if message["kind"] == "points":
        return desugar_points(message)
    if message["kind"] == "transform":
        return canonicalize_transform(message)
    raise NdselError(Reason.UNKNOWN_KIND, f"unknown kind: {message['kind']}")
