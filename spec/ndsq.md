# ndsq — Normative Specification

**Version:** 1.0-draft
**Date:** 2026-06-13

---

## 1. Overview

`ndsq` ("n-dimensional spatial query") is a **JSON-serializable representation of array indexing operations**. Given an n-dimensional integer index space, an ndsq message denotes a **subset of points together with how those points are arranged in a result array**.

ndsq is **denotational, not operational**: a message encodes a resolved selection-and-arrangement (the resulting domain and output maps), never a chained sequence of indexing operations such as `transpose`, `newaxis`, or `vindex`. The effect of any such operation is baked into the resolved message prior to serialization.

To express the same selection compactly at varying cost, ndsq defines a **shorthand ladder** of message kinds ordered from compact to explicit:

| `kind`      | Selects                              |
|-------------|--------------------------------------|
| `point`     | A single point; 0-D scalar result    |
| `box`       | A contiguous hyperrectangle; n-D result |
| `slice`     | A regular strided region; n-D result |
| `points`    | An arbitrary explicit set; 1-D result |
| `transform` | Anything (the full canonical core)   |

Every shorthand MUST have a normative desugaring to `transform` (see §5). `transform` is the universal escape hatch: any selection-with-arrangement is representable. The shorthands are faithful special cases chosen to cover common selection shapes compactly.

---

## 2. Relationship to tensorstore

ndsq **adapts the index model of Google's [tensorstore](https://google.github.io/tensorstore/)**. tensorstore's `IndexTransform` — an input domain plus a closed set of output index-map kinds — covers affine slicing, transposition, new axes, and arbitrary point selection uniformly. ndsq takes this model as its rigorous canonical core.

ndsq departs from tensorstore in exactly two ways:

1. **The `kind` discriminator.** Every ndsq message carries a `kind` string field. tensorstore's native JSON has no such field. Strip `kind` from a normalized `transform` and the result is a tensorstore-loadable `IndexTransform` (subject to any tensorstore-specific parsing quirks). This is the only structural difference in the canonical form.

2. **Coordinate-frame preservation (World A).** ndsq preserves the **source** coordinate frame of each result dimension by default. Re-basing a dimension to origin 0 MUST be done explicitly via a `transform`; no shorthand does it implicitly. Consequently, a pure restriction (a `box`) desugars to **identity** output maps — it performs no rearrangement and therefore introduces none.

ndsq keeps tensorstore's exact field names for the `transform` kind for documentary fidelity. The shorthands are ndsq's own.

---

## 3. Conventions

### 3.1 The `kind` discriminator

Every ndsq message MUST be a JSON object with a `kind` field whose value is a non-empty string. Implementations MUST reject messages that are not objects or that lack `kind`. An unknown `kind` value MUST be rejected with reason code `unknown_kind` (see §7).

### 3.2 Infinity sentinels

The strings `"-inf"` and `"+inf"` (exactly those characters, including the sign) represent negative and positive infinity respectively in bound arrays. Implementations MUST accept them wherever a bound integer is legal. Implementations MUST NOT accept bare numeric infinity (e.g., JSON does not define `Infinity`).

### 3.3 Implicit bounds: the `[n]`-bracket convention

A bound value wrapped in a single-element JSON array — `[n]` or `["-inf"]` or `["+inf"]` — denotes an **implicit** bound. An implicit bound carries the same numeric value as the bare form but signals that the dimension size is not fixed by the message; a consumer may substitute the actual array extent. A bare value (not in an array) denotes an **explicit** bound.

### 3.4 Rank

The input rank and output rank of any `transform` MUST each be a non-negative integer. The 32-dimension ceiling is a tensorstore *implementation* constraint, not a property of this format: implementations that interoperate with tensorstore SHOULD reject ranks greater than 32, but ndsq itself imposes no upper bound.

### 3.5 Integer value range

All coordinate and bound values (excluding the `"-inf"`/`"+inf"` sentinels) are **64-bit signed integers**. Values outside `[-2^63, 2^63 - 1]` are out of range; implementations on platforms with unbounded integers (e.g. Python) MUST treat the 64-bit range as the canonical contract so that all conformant implementations agree.

---

## 4. The canonical core: `kind: "transform"`

### 4.1 Accepted form

A message with `kind: "transform"` is an object with the following members. All members except `kind` follow tensorstore's `IndexTransform` JSON encoding.

#### Input domain

- `input_rank` — a non-negative integer (see §3.4 on the advisory 32-dimension ceiling). MAY be omitted when it can be inferred unambiguously from the lengths of the other arrays; implementations MUST infer it in that case. If present and inconsistent with the other arrays, the message MUST be rejected with reason code `rank_mismatch`.
- `input_inclusive_min` — array of length `input_rank`. Each element is an integer, `"-inf"`, or one of those wrapped in a single-element array (implicit bound). If omitted, defaults to all zeros (explicit).
- **Upper bound** — exactly one of:
  - `input_exclusive_max` — array of length `input_rank`; element `i` is the exclusive upper bound for dimension `i`.
  - `input_inclusive_max` — same structure; the inclusive upper bound.
  - `input_shape` — array of length `input_rank`; element `i` is the size of dimension `i` (`exclusive_max = inclusive_min + shape`).

  Each element uses the same integer / sentinel / bracket convention as `input_inclusive_min`. Providing more than one of these three fields MUST be rejected with reason code `multiple_upper_bounds`.

- `input_labels` — array of strings of length `input_rank`. An empty string `""` denotes an unlabeled dimension. If omitted, all dimensions are unlabeled.

#### Output index maps

- `output` — array of output-map objects of length equal to the output rank. If omitted entirely, the output rank equals the input rank and defaults to the **identity transform** (output map `k` is `single_input_dimension` with `input_dimension=k`, `offset=0`, `stride=1`).

  Each output-map object is one of three kinds, determined by which fields are present:

  - **`constant`** — neither `input_dimension` nor `index_array` is present. Carries `offset` (integer; default `0`). The output coordinate is `offset` regardless of the input.
    - In canonical form: `stride` MUST be omitted (it is meaningless for a constant map).

  - **`single_input_dimension`** (affine) — `input_dimension` is present (a non-negative integer index into the input dimensions). The output coordinate is `offset + stride * input[input_dimension]`. `offset` defaults to `0`; `stride` defaults to `1`.

  - **`index_array`** — `index_array` is present (a nested JSON array of int64 values with rank equal to `input_rank`, broadcast-compatible with the input domain; singleton dimensions are allowed). Optional `index_array_bounds` is a two-element array `[lo, hi]` (both inclusive) giving the valid range of values in the array; defaults to `["-inf", "+inf"]`. The output coordinate is `offset + stride * index_array[input...]`. `offset` defaults to `0`; `stride` defaults to `1`.

### 4.2 Canonical (normalized) form

The function `normalize(message) → transform` MUST accept all redundancies in §4.1 and emit exactly one deterministic spelling. The normalized result is the **bare canonical transform body** — the input domain plus the explicit `output` maps. It does **not** carry a `kind` field: `kind` is a discriminator on *input messages* only, and stripping it is exactly what makes the normalized body a tensorstore-loadable `IndexTransform` (§2). The body MUST contain:

- `input_rank` MUST be present.
- `input_inclusive_min` MUST be present and fully explicit (no omission; defaults written out).
- The upper bound MUST be expressed as `input_exclusive_max` regardless of which spelling the input used.
- `input_labels` MUST be present (all `""` if none were given).
- `output` MUST be present and fully written out (no implicit identity; each output-map entry explicit).
- For `single_input_dimension` and `index_array` maps, `offset` and `stride` MUST be present even when equal to their defaults.
- For `index_array` maps, `index_array_bounds` MUST be present.
- For `constant` maps, only `offset` is emitted; `stride` and `input_dimension` MUST be absent.
- `normalize` MUST be idempotent: `normalize(normalize(x)) == normalize(x)`.

The canonical form is intentionally verbose. Compactness is the responsibility of the shorthands, not the canonical core.

---

## 5. Shorthands and their desugarings

Each shorthand MUST be reducible to a normalized `transform` by the rules below. Implementations MUST be observably equivalent to desugaring before acting.

### 5.1 `point`

```json
{ "kind": "point", "coords": [i0, i1, ..., i_{n-1}] }
```

Selects one cell of an n-dimensional source; the result is a 0-D scalar.

`coords` MUST be an array of n integers.

**Desugaring:** `input_rank: 0`, empty domain (`input_inclusive_min` and `input_exclusive_max` are empty arrays), no `input_labels`. `output` is an array of n `constant` maps: `output[k] = { "offset": coords[k] }`.

### 5.2 `box`

```json
{ "kind": "box", "inclusive_min": [...], "exclusive_max": [...], "labels": [...] }
```

A contiguous hyperrectangle; the result is an n-D array with the same coordinate frame as the source. A `box` is exactly an `IndexDomain` (the input domain of §4.1) paired with identity output, so it accepts the **same bound forms** as a `transform`'s input domain — including infinity sentinels and the `[n]`-bracket implicit form. (`slice`, by contrast, takes plain integer `start`/`stop`/`step` only.)

- `inclusive_min` — array of n bounds, each an integer, `"-inf"`, or one of those wrapped in a single-element array (implicit). MUST be omitted only when `shape` is provided, in which case it defaults to all explicit zeros.
- Upper bound — exactly one of `exclusive_max`, `inclusive_max`, or `shape`, using the same integer / sentinel / bracket convention (same mutual-exclusion rule as §4.1; reject with `multiple_upper_bounds`).
- `labels` — optional array of n strings.

**Desugaring:** `input_rank = n`. `input_inclusive_min` = the box's `inclusive_min`. `input_exclusive_max` = the box's exclusive upper bound (derived from whichever upper-bound field was given). `output[k] = single_input_dimension(input_dimension=k, offset=0, stride=1)` for each k — identity maps, because a pure restriction performs no rearrangement.

### 5.3 `slice`

```json
{ "kind": "slice", "start": [...], "stop": [...], "step": [...], "labels": [...] }
```

A per-dimension regular strided region; the result is an n-D array. `start`, `stop`, and `step` are parallel arrays of length n following Python/NumPy conventions (`stop` is exclusive).

- `start` — array of n integers. MUST be present.
- `stop` — array of n integers. MUST be present.
- `step` — array of n integers. MAY be omitted; defaults to all `1`s. Each element MUST be non-zero; a zero step MUST be rejected with reason code `step_zero`.
- `labels` — optional array of n strings.
- **Negative `step` is reserved.** Implementations MAY reject a negative step value with reason code `negative_step_unsupported` until the negative-step formula is fixed by the conformance corpus.

**Desugaring for `step[k] > 0`.** For each dimension k with `start[k] = a`, `stop[k] = b`, `step[k] = s`:

- `m = max(0, ceil((b - a) / s))` — number of selected points
- `o = floor(a / s)` — result-dimension origin (source origin divided by stride; preserves the source coordinate frame)
- `offset = a - s * o` (equivalently `a mod s`, in `[0, s)`) — the lattice phase, intrinsic to the selection
- Output map for dimension k: `single_input_dimension(input_dimension=k, offset=offset, stride=s)`
- Input domain for dimension k: `[o, o + m)` (i.e., `input_inclusive_min[k] = o`, `input_exclusive_max[k] = o + m`)

The result domain is `[o, o + m)` in source coordinates, **not** re-based to `[0, m)`. To obtain a 0-origin result, apply an explicit translation via `transform`.

### 5.4 `points`

```json
{ "kind": "points", "coords": [[...], [...], ...] }
```

An arbitrary explicit set of m points in row-major form (each inner array is one n-D point). The result is a 1-D array of length m indexed `[0, m)`.

- `coords` MUST be an array of m arrays each of length n, where n is the source rank (may be 0 for scalar; m MUST then be 0 or 1).
- All integers; no sentinels or brackets.

**Desugaring:** `input_rank: 1`, input domain `[0, m)` (`input_inclusive_min: [0]`, `input_exclusive_max: [m]`). `output` is an array of n `index_array` maps. The row-major coordinate list is transposed to tensorstore's columnar form: `output[k] = index_array(index_array=[coords[0][k], coords[1][k], ..., coords[m-1][k]], offset=0, stride=1, index_array_bounds=["-inf", "+inf"])`.

---

## 6. Error codes

Implementations MUST produce exactly one of the following reason codes when rejecting an input, and MUST NOT produce a reason code for a valid input:

| Code                        | Condition                                                                 |
|-----------------------------|---------------------------------------------------------------------------|
| `invalid_json`              | The input is not valid JSON, is not a JSON object, lacks a `kind` field, or has fields whose types do not match this specification. |
| `unknown_kind`              | The `kind` field is present but its value is not a recognized kind string. |
| `multiple_upper_bounds`     | More than one of `exclusive_max`/`inclusive_max`/`shape` is present in a `transform` or `box`. |
| `rank_mismatch`             | `input_rank` is present and inconsistent with the lengths of other arrays, or arrays of inconsistent lengths are provided. |
| `step_zero`                 | A `slice` message contains a `step` value of `0`.                        |
| `negative_step_unsupported` | A `slice` message contains a negative `step` value (until the negative-step formula is specified). |

---

## 7. Conformance

An implementation is **conformant** if and only if:

1. Its `normalize` function reproduces the `normalized` value from every success fixture in `/conformance/` via structural JSON equality.
2. Its `normalize` function rejects every error fixture in `/conformance/` with the reason code given by the fixture's `error` field.

Fixtures have the forms:

```json
{ "input": <any ndsq message>, "normalized": <canonical transform body, without `kind`> }
{ "input": <any ndsq message>, "error": "<reason-code>" }
```

The `normalized` value is the bare canonical transform body of §4.2 (no `kind`).

Three independent implementations passing the same corpus is the primary evidence that this specification is unambiguous and implementable.

---

## 8. Out of scope

The following are explicitly outside the scope of ndsq v1:

- **Chunk decomposition.** ndsq models selection over the logical array index space. Pushing a selection down into per-chunk selections is an implementation concern of the consumer.
- **Data values.** ndsq describes which points are selected and how they are arranged in the result; it does not describe array contents.
- **Operation chaining.** Messages are denotational; there is no on-the-wire composition or sequencing of ndsq messages.

---

*This spec adapts the index model of Google tensorstore; see the design doc for rationale.*
