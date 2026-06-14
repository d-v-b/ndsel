"""Desugaring of the four shorthand kinds to a canonical Transform."""

from __future__ import annotations

from .domain import Domain, canonicalize_domain
from .errors import NdsqError, Reason
from .output import ConstantMap, IndexArrayMap, SingleInputDimension
from .transform import Transform
from .values import ImplicitValue


def desugar_point(msg: dict) -> Transform:
    coords = msg["coords"]
    domain = Domain(rank=0, inclusive_min=[], exclusive_max=[], labels=[])
    output = [ConstantMap(offset=c) for c in coords]
    return Transform(domain=domain, output=output)


def desugar_box(msg: dict) -> Transform:
    domain = canonicalize_domain(
        inclusive_min=msg.get("inclusive_min"),
        exclusive_max=msg.get("exclusive_max"),
        inclusive_max=msg.get("inclusive_max"),
        shape=msg.get("shape"),
        labels=msg.get("labels"),
    )
    output = [SingleInputDimension(offset=0, stride=1, input_dimension=k) for k in range(domain.rank)]
    return Transform(domain=domain, output=output)


def desugar_slice(msg: dict) -> Transform:  # implemented in Task 9
    raise NotImplementedError("desugar_slice — Task 9")


def desugar_points(msg: dict) -> Transform:  # implemented in Task 10
    raise NotImplementedError("desugar_points — Task 10")
