"""Desugaring of the four shorthand kinds to a canonical Transform."""

from __future__ import annotations

from .domain import Domain, canonicalize_domain
from .errors import NdselError, Reason
from .messages import BoxMessage, PointMessage, PointsMessage, SliceMessage
from .output import ConstantMap, IndexArrayMap, OutputMap, SingleInputDimension
from .transform import Transform
from .values import MaybeImplicitValue, require_int_list, require_list, require_str_list


def desugar_point(msg: PointMessage) -> Transform:
    coords = require_int_list(msg.get("coords"), "point.coords")
    domain = Domain(rank=0, inclusive_min=[], exclusive_max=[], labels=[])
    output = [ConstantMap(offset=c) for c in coords]
    return Transform(domain=domain, output=output)


def desugar_box(msg: BoxMessage) -> Transform:
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


def _trunc_div(a: int, s: int) -> int:
    """Integer a / s truncated toward zero (Python's // is floor, not trunc)."""
    q = a // s
    if q < 0 and q * s != a:
        q += 1
    return q


def desugar_slice(msg: SliceMessage) -> Transform:
    start = require_int_list(msg.get("start"), "slice.start")
    stop = require_int_list(msg.get("stop"), "slice.stop")
    rank = len(start)
    if len(stop) != rank:
        raise NdselError(Reason.RANK_MISMATCH, "start and stop must have equal length")
    raw_step = msg.get("step")
    if raw_step is None:
        step = [1] * rank
    else:
        step = require_int_list(raw_step, "slice.step")
        if len(step) != rank:
            raise NdselError(Reason.RANK_MISMATCH, "step length must match start/stop")
    raw_labels = msg.get("labels")
    if raw_labels is None:
        labels = None
    else:
        labels = require_str_list(raw_labels, "slice.labels")
        if len(labels) != rank:
            raise NdselError(Reason.RANK_MISMATCH, "labels length must match start/stop")

    inclusive_min: list[MaybeImplicitValue] = []
    exclusive_max: list[MaybeImplicitValue] = []
    output: list[OutputMap] = []
    for k in range(rank):
        a, b, s = start[k], stop[k], step[k]
        if s == 0:
            raise NdselError(Reason.STEP_ZERO, "step must be non-zero")
        if s < 0:
            raise NdselError(Reason.NEGATIVE_STEP_UNSUPPORTED, "negative step is not yet specified")
        m = 0 if b <= a else _ceil_div(b - a, s)
        o = _trunc_div(a, s)  # trunc(a/s): TensorStore parity (see spec 5.3)
        offset = a - s * o  # lattice phase in (-s, s), sign of the remainder
        inclusive_min.append(MaybeImplicitValue.explicit(o))
        exclusive_max.append(MaybeImplicitValue.explicit(o + m))
        output.append(SingleInputDimension(offset=offset, stride=s, input_dimension=k))

    out_labels = labels if labels is not None else ["" for _ in range(rank)]
    domain = Domain(rank=rank, inclusive_min=inclusive_min, exclusive_max=exclusive_max, labels=out_labels)
    return Transform(domain=domain, output=output)


def desugar_points(msg: PointsMessage) -> Transform:
    raw_coords = require_list(msg.get("coords"), "points.coords")
    coords = [require_int_list(p, f"points.coords[{i}]") for i, p in enumerate(raw_coords)]
    m = len(coords)
    n = len(coords[0]) if coords else 0
    for point in coords:
        if len(point) != n:
            raise NdselError(Reason.RANK_MISMATCH, "all points must have equal dimensionality")

    domain = Domain(
        rank=1,
        inclusive_min=[MaybeImplicitValue.explicit(0)],
        exclusive_max=[MaybeImplicitValue.explicit(m)],
        labels=[""],
    )
    output: list[OutputMap] = []
    for k in range(n):
        column = [coords[i][k] for i in range(m)]
        output.append(IndexArrayMap(offset=0, stride=1, index_array=column, bounds=("-inf", "+inf")))
    return Transform(domain=domain, output=output)
