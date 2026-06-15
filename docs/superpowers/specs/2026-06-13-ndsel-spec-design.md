# ndsel — design spec

**Status:** design (pre-implementation)
**Date:** 2026-06-13
**Topic:** JSON-serializable representation of n-dimensional spatial queries (array indexing/selection-with-arrangement)

---

## 1. Purpose

`ndsel` ("n-dimensional spatial query") defines a **JSON-serializable representation of array indexing operations**: given an n-dimensional integer index space, a message denotes a **subset of points together with how those points are arranged in a result array**.

The same selection can often be expressed many ways. An explicit enumeration of points is always available but expensive (it scales with the number of points selected). So `ndsel` provides a small ladder of **shorthand message formats** that express structured selections compactly, falling back to explicit enumeration only when the selection has no exploitable structure.

The primary consumers are libraries that operate on chunked arrays (Zarr-like). A consumer is expected to be competent to push a logical-space selection down into per-chunk selections; **chunk decomposition is therefore out of scope for this spec** (see §9).

## 2. Relationship to tensorstore

`ndsel` **adapts the index model of Google's [tensorstore](https://google.github.io/tensorstore/)**. tensorstore's `IndexTransform` is, in our assessment, the most complete and well-designed model for n-dimensional indexing: an input domain plus a small, closed set of output index-map kinds covers affine slicing, transposition, new axes, and arbitrary point selection uniformly.

We verified that tensorstore's `IndexTransform`/`IndexDomain` **JSON serialization is not used by any project outside tensorstore itself** (no RFC, no ZEP, no third-party adopter; the only other occurrences of its field names are its author's documentation tooling). The deployed adjacent formats — OPeNDAP constraint expressions, HDF5/netCDF `start/count/stride` hyperslabs — only express regular strided subsetting and are not JSON.

Consequences:

- There is **no interoperability prize** for being byte-compatible with tensorstore's JSON. We are defining a new format regardless.
- What is worth copying is tensorstore's **data model** (the affine / `index_array` output-map taxonomy, implicit bounds, labels), not slavishly its wire ergonomics.
- We therefore take tensorstore's model as our **rigorous canonical core** and give it an **ergonomic tagged-union surface** (§4). The canonical form (§5) keeps tensorstore's exact field names for documentary fidelity; the shorthands (§6) are our own.

The README and spec must state prominently that `ndsel` adapts tensorstore, and document the two deliberate departures (the added `kind` discriminator, §5.1; coordinate-frame preservation as the default, §3).

## 3. Core principles

1. **One canonical core, several shorthands.** Every message is a JSON object with a `kind` discriminator. The canonical `kind: "transform"` is tensorstore's `IndexTransform`. The shorthands (`point`, `box`, `slice`, `points`) are compact encodings, **each defined by a normative desugaring to `transform`** (§6). The canonical core is the single source of truth for semantics.

2. **Denotational, not operational.** A message denotes a *resolved* selection-with-arrangement — the resulting domain and output maps — **not** a chained sequence of indexing operations. This mirrors tensorstore, which serializes only resolved transforms, never `DimExpression` chains. Operations like `transpose`, `newaxis`, `diagonal`, `translate`, `label`, and the `vindex`/`oindex` modes need no dedicated representation: their effect is already baked into the resolved `transform`.

3. **Coordinate-frame preservation (World A).** A selection preserves the **source** coordinate frame of the result dimensions by default. Erasing frame information (re-basing a dimension to origin 0, NumPy-style) must be done **explicitly** (e.g. a `translate_to`-style transform), never implicitly by a shorthand. This keeps each output-map field with exactly one meaning:
   - **domain** = what is selected,
   - **stride** = subsampling,
   - **input-dimension permutation** = reordering,
   - **offset** = translation (and *only* when a translation is meant; the one exception is the lattice phase of a strided slice, §6.3, which is intrinsic to the selection).

   The decisive consequence: a pure restriction (a `box`) desugars to **identity** output maps — it performs no rearrangement, so it introduces none. A re-basing convention would instead represent "select this region" as "translate these points," conflating selection with transformation. We reject that.

4. **Full generality in the core.** The canonical `transform` supports the complete tensorstore model: `-inf`/`+inf` sentinels, implicit bounds (the `[n]`-bracket convention), per-dimension labels, and all three output-map kinds. Anything tensorstore can express, `ndsel` can express via `transform`.

5. **`normalize` is the conformance contract.** The spec defines `normalize(message) → transform`: desugar any message to the canonical core and emit it in a single deterministic, idempotent canonical spelling (§5.2). Implementations may operate on shorthands directly for efficiency, but must be observably equivalent to "desugar, then act." The cross-language conformance corpus (§8) is built on `normalize`.

## 4. Message taxonomy

A message is a JSON object discriminated by a string `kind` field. The variants, ordered compact → explicit:

| `kind`      | Selects                                   | Desugars to (§6)                       |
|-------------|-------------------------------------------|----------------------------------------|
| `point`     | a single point (0-D scalar result)        | all-`constant` output maps             |
| `box`       | a contiguous hyperrectangle               | `IndexDomain`, identity output         |
| `slice`     | a regular strided region                  | `single_input_dimension` affine maps   |
| `points`    | an arbitrary explicit set of points       | `index_array` output maps              |
| `transform` | anything (the full tensorstore model)     | itself (the canonical core)            |

This set is **expressively complete**: `transform` is the universal escape hatch, so any selection-with-arrangement is representable. The shorthands are faithful special cases of real tensorstore indexing operations, chosen to cover the common *selection shapes* compactly.

**Deferred variants (YAGNI; may be added later without breaking the model):**

- `mask` — a dense boolean array. Pure sugar over `points` (the coordinates of the `True` cells), exactly as tensorstore lowers boolean indexing. Deferred until a use case wants to ship a bitmap rather than enumerated coordinates.
- `translate` — sugar for explicit re-basing (`translate_to`/`translate_by`). The capability already exists via `transform`; only the sugar is deferred.

## 5. The canonical core: `kind: "transform"`

This is tensorstore's `IndexTransform` JSON, adopted verbatim except for the added `kind` discriminator (§5.1).

### 5.1 Accepted form

An object with `kind: "transform"` and these members:

**Input domain:**

- `input_rank` — integer in `[0, 32]` (inferable from the other arrays when present).
- `input_inclusive_min` — array; each element is an integer or `"-inf"` (explicit lower bound), or that value wrapped in a single-element array `[n]` / `["-inf"]` (**implicit** lower bound).
- **Upper bound** — exactly one of:
  - `input_exclusive_max`,
  - `input_inclusive_max`, or
  - `input_shape`;

  same integer / `"+inf"` / `[n]`-bracket convention.
- `input_labels` — array of strings; `""` denotes an unlabeled dimension.

**Output index maps:**

- `output` — array of output-map objects, length = output rank. **If omitted, defaults to the identity transform** over the input domain. Each map is one of three kinds, selected by which members are present:
  - **`constant`** — neither `input_dimension` nor `index_array`. Carries only `offset` (default `0`). `output[j] = offset`. (`stride` is meaningless with no input and is omitted in canonical form.)
  - **`single_input_dimension`** (affine) — has `input_dimension` (integer index into the input dims). `output[j] = offset + stride · input[input_dimension]`. `offset` default `0`, `stride` default `1`.
  - **`index_array`** — has `index_array` (a nested JSON array of int64 with rank = `input_rank`, broadcast-compatible; singleton dims allowed). Optional `index_array_bounds` (`[lo, hi]`, default `["-inf", "+inf"]`) constrains valid values. `output[j] = offset + stride · index_array[input...]`.

**The `kind` field is the only deviation** from tensorstore's native object. A normative note will state: *strip `kind` from a normalized `transform` and you have a tensorstore-loadable `IndexTransform`* (modulo any tensorstore-specific quirks). This is the one place fidelity to tensorstore has documentary value.

### 5.2 Canonical (normalized) form

`normalize()` accepts all of tensorstore's redundancies (any of the three upper-bound spellings, omitted `output`, defaulted `offset`/`stride`) but **emits one deterministic spelling** so corpus comparisons are trivial equality checks:

- Upper bound always emitted as **`input_exclusive_max`**.
- `output` always **explicit** (identity written out; no omission).
- `input_rank` always present.
- Defaults written explicitly **where meaningful**: `offset`/`stride` on affine and `index_array` maps; `index_array_bounds` on `index_array` maps. `constant` maps carry only `offset`.
- `normalize` is **idempotent**: `normalize(normalize(x)) == normalize(x)`.

The canonical form is intentionally verbose/explicit. Compactness is the job of the *shorthands*, not the canonical core.

## 6. Shorthands and desugarings

Each shorthand has a normative reduction to a normalized `transform`. Exact integer formulas and worked examples live in the prose spec and are pinned by the conformance corpus; the rules below are the specification.

### 6.1 `point`

```json
{ "kind": "point", "coords": [i0, i1, …, i_{n-1}] }
```

Selects one cell of an n-dimensional source; the result is a 0-D scalar.

**Desugaring:** `input_rank: 0`, empty domain, `output` = n `constant` maps `{ "offset": i_k }`. Frame-independent (no input domain to preserve).

### 6.2 `box`

```json
{ "kind": "box", "inclusive_min": [...], "exclusive_max": [...], "labels": [...]? }
```

A contiguous hyperrectangle; the result is an n-D array. The bounds use **IndexDomain-style named fields** so the interval semantics live in the field name (tensorstore's convention) — there is no bare `min`/`max` to disambiguate.

- `inclusive_min` plus **exactly one of** `exclusive_max` / `inclusive_max` / `shape` (mirroring §5.1).
- `inclusive_min` may be omitted when `shape` is given, defaulting to `0` (the natural "box of this shape at the origin").
- `labels` optional.
- Implicit bounds and `-inf`/`+inf` are **not** expressible in `box`; use `transform`.

**Desugaring:** an `IndexDomain` with **identity** output — `input_inclusive_min` = the box's min, `input_exclusive_max` = the box's max (after converting whichever upper-bound spelling was used), and `output[k] = single_input_dimension(input_dimension=k, offset=0, stride=1)`. A pure restriction introduces no rearrangement.

### 6.3 `slice`

```json
{ "kind": "slice", "start": [...], "stop": [...], "step": [...]?, "labels": [...]? }
```

A per-dimension regular strided region (parallel arrays). The result is n-D. `start`/`stop`/`step` keep their universal Python/NumPy meaning (`stop` exclusive); this idiom is unambiguous and is **not** renamed to `*_min`/`*_max`.

- `step` optional; defaults to all `1`s. **Negative `step` is allowed** (axis reversal).
- `step` must be non-zero (a semantic constraint enforced by the spec/corpus, not JSON Schema).

**Desugaring (coordinate-preserving, World A).** For each dimension with `start = a`, `stop = b`, `step = s`:

For `s > 0` (the result selects exactly the NumPy point set `{a, a+s, a+2s, …}` strictly below `b`, in the **source** coordinate frame):

- `m = max(0, ceil((b − a) / s))`  — number of selected points
- `o = floor(a / s)`  — result-dimension origin (the source origin divided by the stride: the coordinate-preserving frame)
- `offset = a − s · o`  (equivalently `a mod s`, in `[0, s)`) — the lattice **phase**, intrinsic to the selection, not a free translation
- output map: `single_input_dimension(input_dimension=k, offset=offset, stride=s)`
- input domain for dim k: `[o, o + m)`

For `s < 0`: the mirrored rule (the dimension is reversed). The precise integer formula and rounding for negative `step` are fixed by the conformance corpus.

Note this is **not** re-based to `[0, m)` — that would erase the source frame and is World B, which we rejected. To obtain a 0-origin (NumPy-style) result, apply an explicit `translate_to 0` via `transform`.

> **Implementation note — two tensorstore stride semantics.** tensorstore's NumPy-style `x[a:b:s]` and its `.stride(s)` `DimExpression` differ (the former selects `{a, a+s, …}`; the latter selects multiples of `s` within the range). `ndsel`'s `slice` follows the **NumPy-style** selection (the rule above). This must be stated explicitly and exercised by corpus fixtures.

### 6.4 `points`

```json
{ "kind": "points", "coords": [[...], [...], …] }
```

An arbitrary explicit set of `m` points, **row-major** (each inner array is one n-D point). The result is a 1-D array of length `m`. A freshly enumerated set has no source frame to preserve, so its natural frame **is** `0..m`.

**Desugaring:** `input_rank: 1`, input domain `[0, m)`, `output` = n `index_array` maps. The row-major coordinate list is **transposed** to tensorstore's columnar form: output map `k` carries `index_array = [coords[0][k], coords[1][k], …, coords[m-1][k]]` (a 1-D int64 array of length `m`), with `offset = 0`, `stride = 1`. This row-major→columnar transpose is the ergonomic win over hand-writing `transform`.

## 7. JSON Schema

A JSON Schema (draft 2020-12) at `/schema/ndsel.schema.json` validates **structure/syntax only**:

- the discriminated union on `kind`,
- field presence and types,
- the `[n]`-bracket implicit-bound form,
- mutual exclusivity of the three upper-bound spellings.

It deliberately does **not** encode semantic constraints (`step ≠ 0`, rank agreement across arrays, `index_array` value bounds, broadcast compatibility). Those cannot be expressed cleanly in JSON Schema and live in the prose spec and the conformance corpus instead.

## 8. Conformance corpus

The behavioral contract is a language-agnostic corpus of JSON fixtures at `/conformance/`:

- **Success fixtures:** `{ "input": <any message>, "normalized": <canonical transform> }`. Every implementation's `normalize()` must reproduce `normalized` exactly (structural equality).
- **Error fixtures:** `{ "input": <message>, "error": "<reason-code>" }` for inputs that must be rejected (e.g. `step = 0`, rank mismatch, multiple upper-bound spellings).

Each implementation ships a small runner that loads `/conformance/*.json` and asserts the contract, so all three languages are validated identically. Three independent implementations passing one corpus is the primary evidence that the spec is unambiguous and implementable.

## 9. Out of scope

- **Chunk decomposition.** `ndsel` models selection over the logical array index space. Pushing a selection down into per-chunk selections is an implementation concern of the consumer.
- **Data values.** `ndsel` describes *which* points and *how arranged*, never the array contents.
- **Operation chaining.** Messages are denotational (§3.2); there is no on-the-wire composition of messages in v1.

## 10. Repository organization

A **monorepo** with three independent, idiomatic implementations bound by the shared conformance corpus. Rust/Python/TypeScript implementations are **peers**, not FFI bindings to a single core — the logic is light (JSON desugaring + normalization), and three from-scratch implementations are more valuable as spec validation than a single shared binary would be. "Reference" means the **spec doc + corpus**, not a master implementation.

```
/spec/            ndsel.md (prose spec), worked examples
/schema/          ndsel.schema.json
/conformance/     *.json fixtures (input → normalized | error)
/rust/ndsel/       idiomatic impl + corpus runner
/python/ndsel/     idiomatic impl + corpus runner
/typescript/      idiomatic impl + corpus runner
README.md         "ndsel adapts tensorstore's index model" + pointers
docs/             design docs (this file)
```

**Git:** one repository at the top level. The nested `.git` directories created by `uv init` / `cargo new` (no commits) are removed.

## 11. Decisions log (resolved during design)

1. **Selection + transform**, not pure selection — output arrangement is required.
2. **Full tensorstore generality** in the core (`-inf`/`+inf`, implicit bounds, labels, all three output-map kinds).
3. **Approach B**: canonical core + named shorthands, over a single canonical-only format — tensorstore's JSON is non-standard, so we optimize our own ergonomics while keeping its model as the rigorous core.
4. **Five variants** locked (`point`, `box`, `slice`, `points`, `transform`); `mask` and `translate` deferred.
5. **Keep tensorstore's field names** for `transform`; document the adaptation prominently.
6. **`input_exclusive_max`** is the canonical upper-bound spelling; canonical form is explicit/verbose.
7. **`box` uses IndexDomain-style named bounds** (`inclusive_min`/`exclusive_max`/…), not bare `min`/`max` — interval semantics in the name.
8. **World A (coordinate-preserving)**: result frames preserve source coordinates; re-basing is explicit. Pure selections (`box`) stay identity maps.
9. **Monorepo, independent peer implementations**, shared corpus; single top-level git.

## 12. Open items to pin during implementation

- Exact integer rounding for **negative-`step`** slices (positive-step rule is specified in §6.3; negative-step rule to be fixed with corpus fixtures).
- The precise set of **error reason-codes** for error fixtures (§8).
- Whether `point` with mixed selected/kept dimensions (partial indexing) needs a shorthand or stays in `transform` (currently `transform`).
