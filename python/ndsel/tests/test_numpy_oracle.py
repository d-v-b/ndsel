"""Validate the shorthand success fixtures against real NumPy indexing.

The conformance corpus proves the three implementations agree *with each other*.
This proves the Python implementation agrees *with NumPy*: for each executable
`point`/`box`/`slice`/`points` fixture, normalizing and executing the transform
(`result[i] = source[T(i)]`) must equal the equivalent native NumPy indexing,
computed **independently** from the original message. Skipped if numpy is absent.
"""

from __future__ import annotations

import itertools
import json
from math import prod
from pathlib import Path

import pytest

from ndsel import normalize, parse
from ndsel.messages import NormalizedTransform, OutputMapDict

np = pytest.importorskip("numpy")

REPO_ROOT = Path(__file__).parents[3]
CORPUS_DIR = REPO_ROOT / "conformance"
CAP = 1000  # skip fixtures whose source indices exceed this (e.g. the 2**60 coord)


def _finite(bound: object) -> int:
    value = bound[0] if isinstance(bound, list) else bound
    if isinstance(value, bool) or not isinstance(value, int):
        raise ValueError(f"non-finite bound: {value!r}")  # ±inf (or anything non-int) → skip
    return value


def _eval_output(omap: OutputMapDict, gi: list[int], mins: list[int]) -> int:
    """The reference backend: one output dimension's source coordinate (see the
    numpy_executor.py example)."""
    offset = omap.get("offset", 0)
    if "index_array" in omap:
        arr = np.asarray(omap["index_array"])
        local = tuple(g - lo for g, lo in zip(gi, mins))
        return offset + omap.get("stride", 1) * int(arr[local])
    if "input_dimension" in omap:
        return offset + omap.get("stride", 1) * gi[omap["input_dimension"]]
    return offset


def _transform_coords(transform: NormalizedTransform) -> tuple[list[tuple[int, ...]], tuple[int, ...]]:
    """Source coordinate per result element, plus the result shape. Raises on inf."""
    mins = [_finite(b) for b in transform["input_inclusive_min"]]
    maxs = [_finite(b) for b in transform["input_exclusive_max"]]
    shape = tuple(hi - lo for lo, hi in zip(mins, maxs))
    coords = [
        tuple(_eval_output(m, [lo + i for lo, i in zip(mins, local)], mins) for m in transform["output"])
        for local in itertools.product(*(range(s) for s in shape))
    ]
    return coords, shape


def _box_bounds(msg: dict) -> tuple[list[int], list[int]]:
    """Resolve a box message's (inclusive_min, exclusive_max) directly — independent
    of normalize(). Raises on inf."""

    def vals(key: str) -> list[int] | None:
        return [_finite(b) for b in msg[key]] if key in msg else None

    imin, emax = vals("inclusive_min"), vals("exclusive_max")
    incmax, shape = vals("inclusive_max"), vals("shape")
    rank = len(next(v for v in (imin, emax, incmax, shape) if v is not None))
    imin = imin if imin is not None else [0] * rank
    if emax is not None:
        maxs = emax
    elif incmax is not None:
        maxs = [v + 1 for v in incmax]
    elif shape is not None:
        maxs = [lo + s for lo, s in zip(imin, shape)]
    else:
        raise ValueError("unbounded domain")
    return imin, maxs


def _native_index(msg: dict) -> tuple:
    """The equivalent native NumPy index, derived directly from the message."""
    kind = msg["kind"]
    if kind == "point":
        return tuple(msg["coords"])
    if kind == "slice":
        start, stop = msg["start"], msg["stop"]
        step = msg.get("step") or [1] * len(start)
        return tuple(slice(a, b, s) for a, b, s in zip(start, stop, step))
    if kind == "box":
        mins, maxs = _box_bounds(msg)
        return tuple(slice(a, b) for a, b in zip(mins, maxs))
    # points: vectorized (coordinate) indexing, one index array per dimension.
    coords = msg["coords"]
    n = len(coords[0]) if coords else 0
    return tuple(np.array([p[k] for p in coords], dtype=int) for k in range(n))


def _shorthand_fixtures() -> tuple[list[dict], list[str]]:
    cases, ids = [], []
    for kind in ("point", "box", "slice", "points"):
        for case in json.loads((CORPUS_DIR / f"{kind}.json").read_text()):
            if "normalized" in case:  # success fixtures only
                cases.append(case)
                ids.append(case["name"])
    return cases, ids


_CASES, _IDS = _shorthand_fixtures()


@pytest.mark.parametrize("case", _CASES, ids=_IDS)
def test_matches_numpy(case: dict) -> None:
    transform = normalize(parse(json.dumps(case["input"]))).to_json()
    out_rank = len(transform["output"])
    if out_rank == 0:
        pytest.skip("0-d / empty selection: nothing to index")
    try:
        coords, result_shape = _transform_coords(transform)
        native_idx = _native_index(case["input"])
    except ValueError:
        pytest.skip("unbounded domain (±inf): not materializable")
    if not coords:
        pytest.skip("empty selection")
    if any(c < 0 or c > CAP for coord in coords for c in coord):
        pytest.skip("source index outside the executable range")

    src_shape = tuple(max(coord[ax] for coord in coords) + 1 for ax in range(out_rank))
    source = np.arange(prod(src_shape)).reshape(src_shape)

    got = np.empty(result_shape, dtype=source.dtype)
    for local, coord in zip(itertools.product(*(range(s) for s in result_shape)), coords):
        got[local] = source[coord]

    native = source[native_idx]
    assert np.array_equal(got, native), f"{case['name']}: ndsel {got!r} != native {native!r}"
