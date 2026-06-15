# ndsel — n-dimensional spatial query

**ndsel** is a JSON-serializable representation of n-dimensional array indexing
operations. A message denotes a *subset of grid points* (which points are
selected from a source array) plus *how those points are arranged in the result
array* (their output coordinate frame). The name stands for "n-dimensional
spatial query".

ndsel adapts the index model of
[TensorStore](https://google.github.io/tensorstore/index_space.html).

## Status

`ndsel` is in draft. Anything can change without warning. Use at your own risk.

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
| [`spec/ndsel.md`](spec/ndsel.md) | Normative specification |
| [`schema/ndsel.schema.json`](schema/ndsel.schema.json) | JSON Schema for all message kinds |
| [`conformance/`](conformance/) | Conformance corpus (valid + invalid fixtures) |
| [`rust/ndsel/`](rust/ndsel/) | Rust reference implementation |
| [`python/ndsel/`](python/ndsel/) | Python reference implementation |
| [`typescript/`](typescript/) | TypeScript reference implementation |

## License

The **specification** ([`spec/`](spec/)), the **JSON Schema** ([`schema/`](schema/)),
and the **conformance corpus** ([`conformance/`](conformance/)) are dedicated to the
public domain under [CC0 1.0 Universal](LICENSE-CC0) — implement them freely, no
attribution required.

The **reference implementations** (`rust/`, `python/`, `typescript/`) are licensed
under either of [Apache License 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT)
at your option (`MIT OR Apache-2.0`). Unless you explicitly state otherwise, any
contribution intentionally submitted for inclusion in the work shall be dual
licensed as above, without any additional terms or conditions.
