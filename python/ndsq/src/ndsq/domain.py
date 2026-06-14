"""The canonical input domain and its construction from raw JSON fields."""

from __future__ import annotations

from dataclasses import dataclass

from .errors import NdsqError, Reason
from .values import ImplicitValue, require_int, require_list, require_str_list


@dataclass
class Domain:
    """Per-dimension [inclusive_min, exclusive_max) plus labels."""

    rank: int
    inclusive_min: list[ImplicitValue]
    exclusive_max: list[ImplicitValue]
    labels: list[str]

    def to_json_fields(self) -> dict:
        return {
            "input_rank": self.rank,
            "input_inclusive_min": [v.to_json() for v in self.inclusive_min],
            "input_exclusive_max": [v.to_json() for v in self.exclusive_max],
            "input_labels": list(self.labels),
        }


def _bump_inclusive_to_exclusive(v: ImplicitValue) -> ImplicitValue:
    value = v.value + 1 if isinstance(v.value, int) else v.value
    return ImplicitValue(value=value, implicit=v.implicit)


def _add_shape(lo: ImplicitValue, sz: ImplicitValue) -> ImplicitValue:
    if isinstance(lo.value, int) and isinstance(sz.value, int):
        value: object = lo.value + sz.value
    elif lo.value == "+inf" or sz.value == "+inf":
        value = "+inf"
    else:  # any remaining -inf operand
        value = "-inf"
    return ImplicitValue(value=value, implicit=lo.implicit)  # type: ignore[arg-type]


def canonicalize_domain(
    *,
    rank: int | None = None,
    inclusive_min: list | None = None,
    exclusive_max: list | None = None,
    inclusive_max: list | None = None,
    shape: list | None = None,
    labels: list | None = None,
) -> Domain:
    """Reduce raw JSON-level domain fields to a canonical Domain."""

    def _bounds(raw: object | None, name: str) -> list[ImplicitValue] | None:
        if raw is None:
            return None
        return [ImplicitValue.from_json(b) for b in require_list(raw, name)]

    imin = _bounds(inclusive_min, "inclusive_min")
    emax = _bounds(exclusive_max, "exclusive_max")
    incmax = _bounds(inclusive_max, "inclusive_max")
    shp = _bounds(shape, "shape")
    labels = require_str_list(labels, "labels") if labels is not None else None
    if rank is not None:
        rank = require_int(rank, "rank")
        if rank < 0:
            raise NdsqError(Reason.INVALID_JSON, f"rank must be non-negative, got {rank}")

    upper_count = (emax is not None) + (incmax is not None) + (shp is not None)
    if upper_count > 1:
        raise NdsqError(
            Reason.MULTIPLE_UPPER_BOUNDS,
            "specify only one of exclusive_max / inclusive_max / shape",
        )

    resolved_rank = rank
    for arr in (imin, emax, incmax, shp, labels):
        if arr is None:
            continue
        if resolved_rank is not None and resolved_rank != len(arr):
            raise NdsqError(
                Reason.RANK_MISMATCH,
                f"array length {len(arr)} disagrees with rank {resolved_rank}",
            )
        resolved_rank = len(arr)
    r = resolved_rank if resolved_rank is not None else 0

    if imin is None:
        imin = [ImplicitValue.explicit(0) for _ in range(r)]

    if emax is not None:
        exclusive = emax
    elif incmax is not None:
        exclusive = [_bump_inclusive_to_exclusive(v) for v in incmax]
    elif shp is not None:
        exclusive = [_add_shape(lo, sz) for lo, sz in zip(imin, shp)]
    else:
        exclusive = [ImplicitValue(value="+inf", implicit=True) for _ in range(r)]

    out_labels = labels if labels is not None else ["" for _ in range(r)]
    return Domain(rank=r, inclusive_min=imin, exclusive_max=exclusive, labels=out_labels)
