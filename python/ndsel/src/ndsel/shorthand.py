"""Desugaring of the five shorthand kinds to a canonical Transform."""

from __future__ import annotations

from .domain import Domain, canonicalize_domain
from .errors import NdselError, Reason
from .messages import BoxMessage, OuterMessage, PointMessage, PointsMessage, SliceMessage
from .output import ConstantMap, IndexArrayMap, OutputMap, SingleInputDimension
from .transform import Transform
from .values import MaybeImplicitValue, require_int, require_int_list, require_list, require_str_list


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


def _slice_dimension(
    a: int, b: int, s: int, k: int
) -> tuple[MaybeImplicitValue, MaybeImplicitValue, SingleInputDimension]:
    """One dimension of the `slice` desugaring (spec 5.3): its domain bounds and map."""
    if s == 0:
        raise NdselError(Reason.STEP_ZERO, "step must be non-zero")
    # Source interval [a, b) for s > 0, [b+1, a+1) for s < 0 — see spec 5.3.
    n = b - a if s > 0 else a - b
    if n < 0:
        raise NdselError(
            Reason.BOUNDS_OUT_OF_ORDER,
            f"dimension {k}: stop must be at or after start in the direction of travel "
            f"(stop >= start for step > 0, stop <= start for step < 0); "
            f"got start={a}, stop={b}, step={s}",
        )
    m = _ceil_div(n, abs(s))
    o = _trunc_div(a, s)  # trunc(a/s), both signs: TensorStore parity (see spec 5.3)
    offset = a - s * o  # lattice phase: |offset| < |s|, sign of a
    return (
        MaybeImplicitValue.explicit(o),
        MaybeImplicitValue.explicit(o + m),
        SingleInputDimension(offset=offset, stride=s, input_dimension=k),
    )


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
        lo, hi, omap = _slice_dimension(start[k], stop[k], step[k], k)
        inclusive_min.append(lo)
        exclusive_max.append(hi)
        output.append(omap)

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


# The members of each `outer` axis-selector form (spec 5.5).
_RANGE_FIELDS = frozenset({"start", "stop", "step"})


def _outer_axis_form(raw: dict[str, object], k: int) -> str:
    """Discriminate an object selector by the members present, then enforce its
    closed member set: indices > intervals > range."""
    form, allowed = (
        ("indices", {"indices"})
        if "indices" in raw
        else ("intervals", {"intervals"})
        if "intervals" in raw
        else ("range", _RANGE_FIELDS)
    )
    extra = set(raw) - allowed
    if extra:
        raise NdselError(
            Reason.UNKNOWN_FIELD,
            f"outer.axes[{k}]: unrecognized field(s) for a {form} selector: "
            f"{', '.join(sorted(map(str, extra)))}",
        )
    return form


def _expand_intervals(raw: object, k: int) -> list[int]:
    coords: list[int] = []
    for j, pair in enumerate(require_list(raw, f"outer.axes[{k}].intervals")):
        what = f"outer.axes[{k}].intervals[{j}]"
        bounds = require_int_list(pair, what)
        if len(bounds) != 2:
            raise NdselError(Reason.INVALID_JSON, f"{what} must be a [lo, hi] pair")
        lo, hi = bounds
        if lo > hi:
            raise NdselError(Reason.BOUNDS_OUT_OF_ORDER, f"{what}: lo {lo} exceeds hi {hi}")
        coords.extend(range(lo, hi))
    return coords


def _broadcast(coords: list[int], rank: int, k: int) -> object:
    """`coords` reshaped to the broadcast shape: `len(coords)` in dimension k, 1 elsewhere."""
    arr: list[object] = []
    for c in coords:
        leaf: object = c
        for _ in range(rank - 1 - k):
            leaf = [leaf]
        arr.append(leaf)
    nested: object = arr
    for _ in range(k):
        nested = [nested]
    return nested


def desugar_outer(msg: OuterMessage) -> Transform:
    axes = require_list(msg.get("axes"), "outer.axes")
    rank = len(axes)
    raw_labels = msg.get("labels")
    if raw_labels is None:
        labels = ["" for _ in range(rank)]
    else:
        labels = require_str_list(raw_labels, "outer.labels")
        if len(labels) != rank:
            raise NdselError(Reason.RANK_MISMATCH, "labels length must match axes")

    inclusive_min: list[MaybeImplicitValue] = []
    exclusive_max: list[MaybeImplicitValue] = []
    output: list[OutputMap] = []
    for k, raw in enumerate(axes):
        if raw is None:
            # The whole axis: implicit infinite bounds, resolved against the array.
            inclusive_min.append(MaybeImplicitValue(value="-inf", implicit=True))
            exclusive_max.append(MaybeImplicitValue(value="+inf", implicit=True))
            output.append(SingleInputDimension(offset=0, stride=1, input_dimension=k))
            continue
        if not isinstance(raw, dict):
            raise NdselError(Reason.INVALID_JSON, f"outer.axes[{k}] must be null or an object, got {raw!r}")
        form = _outer_axis_form(raw, k)
        if form == "range":
            if "start" not in raw or "stop" not in raw:
                raise NdselError(Reason.INVALID_JSON, f"outer.axes[{k}]: a range needs start and stop")
            start = require_int(raw["start"], f"outer.axes[{k}].start")
            stop = require_int(raw["stop"], f"outer.axes[{k}].stop")
            step = require_int(raw["step"], f"outer.axes[{k}].step") if "step" in raw else 1
            lo, hi, omap = _slice_dimension(start, stop, step, k)
            inclusive_min.append(lo)
            exclusive_max.append(hi)
            output.append(omap)
            continue
        if form == "indices":
            coords = require_int_list(raw["indices"], f"outer.axes[{k}].indices")
        else:
            coords = _expand_intervals(raw["intervals"], k)
        inclusive_min.append(MaybeImplicitValue.explicit(0))
        exclusive_max.append(MaybeImplicitValue.explicit(len(coords)))
        # Empty and single-element lists collapse to constant maps (spec 7.1).
        if len(coords) == 0:
            output.append(ConstantMap(offset=0))
        elif len(coords) == 1:
            output.append(ConstantMap(offset=coords[0]))
        else:
            output.append(
                IndexArrayMap(
                    offset=0, stride=1, index_array=_broadcast(coords, rank, k), bounds=("-inf", "+inf")
                )
            )

    domain = Domain(rank=rank, inclusive_min=inclusive_min, exclusive_max=exclusive_max, labels=labels)
    return Transform(domain=domain, output=output)
