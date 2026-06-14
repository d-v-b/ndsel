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


def _ceil_div(p: int, q: int) -> int:
    """Ceiling of p/q for p >= 0, q > 0."""
    return (p + q - 1) // q


def desugar_slice(msg: dict) -> Transform:
    start = msg["start"]
    stop = msg["stop"]
    rank = len(start)
    if len(stop) != rank:
        raise NdsqError(Reason.RANK_MISMATCH, "start and stop must have equal length")
    step = msg.get("step")
    if step is None:
        step = [1] * rank
    elif len(step) != rank:
        raise NdsqError(Reason.RANK_MISMATCH, "step length must match start/stop")
    labels = msg.get("labels")
    if labels is not None and len(labels) != rank:
        raise NdsqError(Reason.RANK_MISMATCH, "labels length must match start/stop")

    inclusive_min: list[ImplicitValue] = []
    exclusive_max: list[ImplicitValue] = []
    output: list = []
    for k in range(rank):
        a, b, s = start[k], stop[k], step[k]
        if s == 0:
            raise NdsqError(Reason.STEP_ZERO, "step must be non-zero")
        if s < 0:
            raise NdsqError(Reason.NEGATIVE_STEP_UNSUPPORTED, "negative step is not yet specified")
        m = 0 if b <= a else _ceil_div(b - a, s)
        o = a // s  # floor(a/s) for s > 0
        offset = a - s * o  # lattice phase in [0, s)
        inclusive_min.append(ImplicitValue.explicit(o))
        exclusive_max.append(ImplicitValue.explicit(o + m))
        output.append(SingleInputDimension(offset=offset, stride=s, input_dimension=k))

    out_labels = labels if labels is not None else ["" for _ in range(rank)]
    domain = Domain(rank=rank, inclusive_min=inclusive_min, exclusive_max=exclusive_max, labels=out_labels)
    return Transform(domain=domain, output=output)


def desugar_points(msg: dict) -> Transform:  # implemented in Task 10
    raise NotImplementedError("desugar_points — Task 10")
