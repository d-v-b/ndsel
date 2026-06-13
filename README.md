# ndsq — n-dimensional spatial query

**ndsq** is a JSON-serializable representation of n-dimensional array indexing
operations. A message denotes a *subset of grid points* (which points are
selected from a source array) plus *how those points are arranged in the result
array* (their output coordinate frame). The name stands for "n-dimensional
spatial query".

ndsq adapts the index model of
[Google TensorStore](https://google.github.io/tensorstore/index_space.html).
For design rationale and the full motivation see
[`docs/superpowers/specs/2026-06-13-ndsq-spec-design.md`](docs/superpowers/specs/2026-06-13-ndsq-spec-design.md).

## Message kinds

There are five message kinds, ranging from convenient shorthands to the full
canonical form.

| Kind | Description | Example |
|------|-------------|---------|
| `point` | A single grid point | `{"kind":"point","coords":[4,7]}` |
| `box` | A contiguous hyperrectangle | `{"kind":"box","inclusive_min":[0,0],"exclusive_max":[3,4]}` |
| `slice` | A regular strided region | `{"kind":"slice","start":[0],"stop":[10],"step":[2]}` |
| `points` | An explicit set of points | `{"kind":"points","coords":[[1,10],[2,20]]}` |
| `transform` | Full canonical core (TensorStore IndexTransform) | `{"kind":"transform","input_inclusive_min":[0,0],"input_exclusive_max":[3,4]}` |

Every message normalizes to the canonical `transform` form. The shorthands are
coordinate-frame preserving: a `box` is an identity transform (output
coordinates equal input coordinates); a `slice` keeps the source coordinate
frame — re-basing the output to 0 is an explicit, separate step.

## Repository layout

| Path | Contents |
|------|----------|
| [`spec/ndsq.md`](spec/ndsq.md) | Normative specification |
| [`schema/ndsq.schema.json`](schema/ndsq.schema.json) | JSON Schema for all message kinds |
| [`conformance/`](conformance/) | Conformance corpus (valid + invalid fixtures) |
| [`rust/ndsq/`](rust/ndsq/) | Rust reference implementation **(implemented)** |
| [`python/ndsq/`](python/ndsq/) | Python implementation (planned — Plan 2) |
| [`typescript/`](typescript/) | TypeScript implementation (planned — Plan 3) |
