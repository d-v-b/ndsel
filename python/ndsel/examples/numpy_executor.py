"""Execute ndsel messages against a real NumPy array, and verify each reproduces
the equivalent *native* NumPy indexing.

This is a tiny **reference backend**: it interprets the canonical Transform the
way a compiled backend (e.g. zarrs) would, but element-by-element in pure Python
for clarity (a real backend vectorizes). It doubles as a check that ndsel
faithfully represents NumPy indexing semantics — the property that makes it
usable as the contract between a NumPy-style frontend (zarr-python) and a
compiled indexing backend (zarrs).

A normalized transform maps an *input domain* (the result's index space) to
*output coordinates* into the source array: result[i] = source[T(i)].

    cd python/ndsel && uv run --with numpy python examples/numpy_executor.py
"""

from __future__ import annotations

import itertools

import numpy as np

from ndsel import Box, Point, Points, Slice, normalize, parse


def _finite(bound: object) -> int:
    """A normalized bound is an int, or a 1-element ``[int]`` (implicit). Executing
    a selection needs finite bounds; ``+/-inf`` domains are not materializable."""
    value = bound[0] if isinstance(bound, list) else bound
    if value in ("-inf", "+inf"):
        raise ValueError("cannot materialize an unbounded domain")
    return int(value)


def _eval_output(omap: dict, gi: list[int], mins: list[int]) -> int:
    """Evaluate one output dimension's source coordinate for input vector ``gi``."""
    offset = omap.get("offset", 0)
    if "index_array" in omap:  # output coordinate looked up from an explicit array
        arr = np.asarray(omap["index_array"])
        local = tuple(g - lo for g, lo in zip(gi, mins))
        return offset + omap.get("stride", 1) * int(arr[local])
    if "input_dimension" in omap:  # affine map of one input dimension
        return offset + omap.get("stride", 1) * gi[omap["input_dimension"]]
    return offset  # constant map (drops an axis at a fixed position)


def execute(transform: dict, source: np.ndarray) -> np.ndarray:
    """Apply a normalized ndsel transform to ``source``; the result's shape is the
    transform's input domain, and ``result[i] = source[T(i)]``."""
    mins = [_finite(b) for b in transform["input_inclusive_min"]]
    maxs = [_finite(b) for b in transform["input_exclusive_max"]]
    shape = [hi - lo for lo, hi in zip(mins, maxs)]
    omaps = transform["output"]
    result = np.empty(shape, dtype=source.dtype)
    for local in itertools.product(*(range(s) for s in shape)):
        gi = [lo + idx for lo, idx in zip(mins, local)]
        coord = tuple(_eval_output(m, gi, mins) for m in omaps)
        result[local] = source[coord]
    return result


def check(label: str, message: object, source: np.ndarray, native: np.ndarray) -> None:
    got = execute(normalize(message).to_json(), source)
    if not np.array_equal(got, native):
        raise AssertionError(f"{label}: ndsel {got!r} != native {native!r}")
    print(f"  OK  {label:34} -> shape {str(tuple(got.shape)):10} matches native")


def main() -> None:
    a = np.arange(5 * 12).reshape(5, 12)
    print("source: a = np.arange(60).reshape(5, 12)\n")

    # --- shorthands vs. the NumPy indexing they stand for ---
    check("point  == a[4, 7]", Point(coords=[4, 7]), a, a[4, 7])
    check("box    == a[0:3, 0:4]", Box(shape=[3, 4]), a, a[0:3, 0:4])
    check("slice  == a[0:5:2, 1:12:3]", Slice(start=[0, 1], stop=[5, 12], step=[2, 3]), a, a[0:5:2, 1:12:3])

    # points == coordinate (vectorized) indexing: a[[1,3], [2,4]]
    pts = [[1, 2], [3, 4]]
    rows = [p[0] for p in pts]
    cols = [p[1] for p in pts]
    check("points == a[[1,3], [2,4]]", Points(coords=pts), a, a[rows, cols])

    # --- the differentiator: arrangement (needs the full `transform` kind) ---
    # A transpose — no shorthand expresses it; the output maps reorder the axes.
    transpose = parse(
        '{"kind":"transform","input_inclusive_min":[0,0],"input_exclusive_max":[12,5],'
        '"output":[{"input_dimension":1},{"input_dimension":0}]}'
    )
    check("transform == a.T (transpose)", transpose, a, a.T)

    print("\nAll ndsel messages reproduced native NumPy indexing exactly.")


if __name__ == "__main__":
    main()
