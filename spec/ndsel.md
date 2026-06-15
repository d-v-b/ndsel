# ndsel — Normative Specification

**Version:** 1.0-draft
**Date:** 2026-06-15

---

## 1. Overview

`ndsel` ("n-dimensional selection") is a **JSON-serializable representation of NumPy-style n-dimensional array indexing** — the indexing written as `a[3]`, `a[2:10:2]`, or `a[[1, 4, 7]]`. It turns such an expression — normally code, evaluated in-process against a single array — into **data**: one language-neutral message that can be serialized, stored, sent across a process or language boundary, and applied to any array of matching rank.

Conceptually, every index over an n-dimensional integer grid has two parts:

1. **The input selection** — *which* points of the source grid are chosen: a contiguous block, a strided lattice, an arbitrary set of points.
2. **The output arrangement** — *how* the chosen points are laid out in the new result array.

The arrangement is a genuine choice, not a formality. The same points may be kept as an n-D sub-array that preserves their relative positions — what *basic slicing* does (`a[2:5, 1:4]` → a 3×3 block) — or **linearized** into a 1-D list — what *advanced indexing* does (`a[[2,3,4], [1,2,3]]` → the points `(2,1), (3,2), (4,3)` as a length-3 vector). NumPy folds this choice into the indexing style; ndsel states it explicitly. (In the canonical form, these two parts are the transform's **input domain** and **output maps**, [section 4](#4-the-canonical-core-kind-transform).)

This is *selection*, not a transformation of the source data: a selected point keeps its source coordinate — selecting picks out existing points, it does not renumber them into a fresh index space ([section 2.2](#22-selected-points-keep-their-source-coordinates)). What a message lays out is the *result*: how the selected points fill the new array.

The canonical representation is the `transform` kind ([section 4](#4-the-canonical-core-kind-transform)), borrowed from tensorstore's `IndexTransform`. For the common kinds (`point`, `box`, `slice`, `points`) the arrangement is the natural one — the selected points in their source order; genuine *rearrangement* (reordering axes, inserting degenerate ones) is available only through `transform`, and the shorthands never rearrange.

ndsel is **denotational, not operational**: a message encodes a *resolved* selection (the points selected and their layout), never a chained sequence of indexing operations such as `transpose`, `newaxis`, or `vindex`. The effect of any such operation is baked into the message before serialization.

To express the same selection compactly at varying cost, ndsel defines a **shorthand ladder** of message kinds ordered from compact to explicit:

| `kind`      | Selects                                  | Result |
|-------------|------------------------------------------|--------|
| `point`     | A single point                           | 0-D scalar |
| `box`       | A contiguous hyperrectangle              | n-D |
| `slice`     | A regular strided region                 | n-D |
| `points`    | An arbitrary explicit set of points      | 1-D |
| `transform` | Anything (the full canonical core)       | any |

Every shorthand MUST have a normative desugaring to `transform` ([section 5](#5-shorthands-and-their-desugarings)). `transform` is the universal escape hatch: any selection is representable (including the rare cases that also rearrange axes). The shorthands are faithful special cases chosen to cover common selection shapes compactly.

### 1.1 A worked example

> *"Select every other element of a length-20 1-D array, starting at index 10."* — in NumPy, `a[10:20:2]`.

That intent — selecting source indices 10, 12, 14, 16, 18 — is a strided `slice`:

```json
{ "kind": "slice", "start": [10], "stop": [20], "step": [2] }
```

Normalizing it ([section 4.3](#43-canonical-normalized-form)) produces the canonical `transform`: the same selection written in the universal core.

```json
{
  "input_rank": 1,
  "input_inclusive_min": [5],
  "input_exclusive_max": [10],
  "input_labels": [""],
  "output": [ { "offset": 0, "stride": 2, "input_dimension": 0 } ]
}
```

The five selected points are `{10, 12, 14, 16, 18}`. In the canonical form the result index `i` names the source point `0 + 2·i` over the domain `[5, 10)` — so the points are addressed `5…9` (point 10 sits at `10 / 2 = 5`). Selecting does **not** renumber the result to a fresh `0…4` array: the selected points keep their source coordinates, so the result stays anchored to the source frame ([section 2.2](#22-selected-points-keep-their-source-coordinates)). A consumer that wants a 0-based result re-bases it explicitly.

The same array can be addressed with the other shorthands, each compact for its shape:

| Intent | Message |
|--------|---------|
| The first 100×100 block of a 2-D array | `{ "kind": "box", "shape": [100, 100] }` |
| The single cell at (3, 5) | `{ "kind": "point", "coords": [3, 5] }` |
| Exactly these three scattered cells | `{ "kind": "points", "coords": [[0,0],[5,2],[9,9]] }` |

---

## 2. Relationship to tensorstore

ndsel **adapts the index model of Google's [tensorstore](https://google.github.io/tensorstore/)**. tensorstore's `IndexTransform` — an input domain plus a closed set of output index-map kinds — covers affine slicing, transposition, new axes, and arbitrary point selection uniformly. ndsel takes this model as its rigorous canonical core and keeps tensorstore's **exact field names** for the `transform` kind (`input_inclusive_min`, `input_exclusive_max`, `output`, `offset`, `stride`, `input_dimension`, `index_array`, `index_array_bounds`, …).

### 2.1 The one structural difference: the `kind` discriminator

ndsel is a **tagged union** of message kinds (`point`/`box`/`slice`/`points`/`transform`), so every message carries a `kind` string field. tensorstore's `IndexTransform` is a single type with no such field. This is the **only** structural difference between an ndsel `transform` and a tensorstore `IndexTransform`:

- **ndsel → tensorstore:** the normalized `transform` body ([section 4.3](#43-canonical-normalized-form)) is, field-for-field, a tensorstore `IndexTransform` *except* for the `kind` member. Because tensorstore's JSON binding is strict about unrecognized members, **strip `kind`** at the boundary, after which the body loads as an ordinary `IndexTransform`.
- **tensorstore → ndsel:** a raw `IndexTransform` has no `kind`; **add `kind: "transform"`** to present it to ndsel's `parse`.

The shorthands (`point`/`box`/`slice`/`points`) are ndsel's own; tensorstore has no JSON encoding for them. They merely desugar to ordinary `transform`s.

### 2.2 Selected points keep their source coordinates

Because a message **selects** existing points rather than transforming them, a selected point keeps the coordinate it had in the source. The result is therefore *anchored to the source frame*, not renumbered to a fresh 0-based index space: selecting `[5, 10)` keeps the domain `[5, 10)`, it is **not** re-based to `[0, 5)`. (This is also tensorstore's convention; the renumbering ndsel avoids is **NumPy's**, which re-bases sliced axes to origin 0.) Re-basing a result to origin 0 MUST be requested explicitly via a `transform`; no shorthand does it implicitly.

A direct consequence: a `box` — which selects a contiguous block and reorders nothing — desugars to **identity** output maps. The points it selects are exactly where they were, so the canonical form is a pure domain restriction with no coordinate mapping at all. Genuine *rearrangement* (reordering or inserting axes) is available, but only through the `transform` kind; the shorthands never rearrange.

---

## 3. Conventions and value types

### 3.1 The `kind` discriminator

Every ndsel message MUST be a JSON object with a string `kind` field. Implementations MUST reject input that is not a JSON object, or that lacks a string `kind`, with reason code `invalid_json`. A `kind` value that is a string but not one of the five recognized kinds (including the empty string) MUST be rejected with `unknown_kind` ([section 6](#6-error-codes)).

### 3.2 Infinity sentinels

The strings `"-inf"` and `"+inf"` (exactly those characters, including the sign) represent negative and positive infinity in **bound** positions (domain bounds and `index_array_bounds`). Implementations MUST accept them wherever a bound is legal, and MUST NOT accept bare numeric infinity (JSON has no `Infinity`). Sentinels are **not** legal in plain-integer positions (`coords`, `start`, `stop`, `step`, `input_dimension`, `input_rank`).

### 3.3 Implicit bounds: the `[n]`-bracket convention

A bound wrapped in a single-element JSON array — `[n]`, `["-inf"]`, or `["+inf"]` — denotes an **implicit** bound. It carries the same value as the bare form but signals that the dimension's extent is not fixed by the message; a consumer MAY substitute the actual array extent. A bare value denotes an **explicit** bound. The implicit/explicit distinction is preserved through normalization ([section 4.3](#43-canonical-normalized-form)).

### 3.4 Rank

The input rank of a `transform` is a non-negative integer. The 32-dimension ceiling is a tensorstore *implementation* constraint, not a property of this format: implementations that interoperate with tensorstore SHOULD reject ranks greater than 32, but ndsel itself imposes no upper bound.

### 3.5 Integer value range

Every coordinate and bound value (excluding the `"-inf"`/`"+inf"` sentinels) MUST be a **64-bit signed integer**, in `[-2^63, 2^63 − 1]`. An implementation MUST accept the full range **exactly** and MUST reject an input integer outside it with `invalid_json`. How values are represented internally is an implementation choice: an implementation whose default numeric type cannot hold all 64-bit integers — notably JavaScript, whose `number` is an IEEE-754 double exact only to ±(2^53 − 1) — MUST use a wider representation (e.g. `BigInt`) for values beyond it, so that no in-range value loses precision. (Integers wider than 64-bit MAY be supported in a future version.)

### 3.6 JSON value types

These named types are referenced by the field tables in [section 4](#4-the-canonical-core-kind-transform)–[section 5](#5-shorthands-and-their-desugarings).

| Type | JSON form |
|------|-----------|
| `integer` | A JSON number with no fractional part, in 64-bit signed range ([section 3.5](#35-integer-value-range)). A JSON boolean is **not** an `integer`. |
| `index-value` | An `integer`, or the string `"-inf"` or `"+inf"`. |
| `bound` | An `index-value` (explicit), or a one-element array `[index-value]` (implicit, [section 3.3](#33-implicit-bounds-the-n-bracket-convention)). |
| `integer[]`, `bound[]`, `string[]`, `integer[][]` | JSON arrays of the named element type. |
| `output-map` | An object as defined in [section 4.2](#42-output-maps). |

A value whose JSON type does not match the type required by a field MUST be rejected with `invalid_json` ([section 6](#6-error-codes)).

---

## 4. The canonical core: `kind: "transform"`

### 4.1 The `transform` message

A `transform` message is a JSON object. All members except `kind` follow tensorstore's `IndexTransform` JSON encoding.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `kind` | `"transform"` | yes | Discriminator. |
| `input_rank` | `integer` ≥ 0 | no | The input rank. MAY be omitted and inferred from the lengths of the bound/label arrays; **defaults to `0`** when no such array is present. If present and inconsistent with any array length → `rank_mismatch`. |
| `input_inclusive_min` | `bound[]` | no | Per-dimension inclusive lower bound; length = rank. If omitted, defaults to all explicit `0`. |
| `input_exclusive_max` | `bound[]` | no † | Per-dimension exclusive upper bound; length = rank. |
| `input_inclusive_max` | `bound[]` | no † | Inclusive upper bound; converted to exclusive as `value + 1`. |
| `input_shape` | `bound[]` | no † | Per-dimension size; converted as `exclusive_max = inclusive_min + shape`. |
| `input_labels` | `string[]` | no | Per-dimension labels; length = rank. `""` = unlabeled. If omitted, all `""`. |
| `output` | `output-map[]` | no | One map per **output** dimension. If omitted, defaults to the **identity** transform (one `single_input_dimension` map per input dimension with `input_dimension = k`, `offset = 0`, `stride = 1`). |

† **Upper bound.** `input_exclusive_max`, `input_inclusive_max`, and `input_shape` are mutually exclusive: **at most one** may appear. Providing two or more → `multiple_upper_bounds`. If **none** is provided, the upper bound defaults to an **implicit `+inf`** in every dimension.

### 4.2 Output maps

An output map is **not tagged**. Unlike a message (which carries a `kind`), a map's kind is determined by **which fields are present**. This matches tensorstore's `IndexTransform` encoding, so the canonical body stays a loadable `IndexTransform` ([section 2.1](#21-the-one-structural-difference-the-kind-discriminator)); and it is unambiguous because the three kinds are distinguished by genuinely different content — a map either consumes an input dimension, or looks up an array, or is constant — so the distinguishing field *is* the natural marker rather than a redundant tag. A map MUST NOT carry both `input_dimension` and `index_array`.

The kind is selected by this rule, in order:

| If the map has… | …its kind is | Output coordinate |
|-----------------|--------------|-------------------|
| `index_array` | **`index_array`** | `offset + stride · index_array[input…]` |
| otherwise `input_dimension` | **`single_input_dimension`** | `offset + stride · input[input_dimension]` |
| otherwise (neither) | **`constant`** | `offset` |

Each kind's fields:

**`constant`** — a fixed output coordinate. The smallest valid output map is `{}` (a constant at offset 0).

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `offset` | `integer` | no | Default `0`. The output coordinate. |

**`single_input_dimension`** — an affine map of one input dimension.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `input_dimension` | `integer` ≥ 0 | **yes** | Index of the input dimension. Its presence selects this kind. |
| `offset` | `integer` | no | Default `0`. |
| `stride` | `integer` | no | Default `1`. |

**`index_array`** — the output coordinate is looked up from an explicit array.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `index_array` | nested array of `integer` | **yes** | Its presence selects this kind. Carried verbatim; see [section 7](#7-validation-deferred-in-v1) on deferred validation. |
| `offset` | `integer` | no | Default `0`. |
| `stride` | `integer` | no | Default `1`. |
| `index_array_bounds` | `[index-value, index-value]` | no | `[lo, hi]`, both inclusive; default `["-inf", "+inf"]`. |

### 4.3 Canonical (normalized) form

`normalize(message) → transform` MUST accept all the redundancies of [section 4.1](#41-the-transform-message)–[section 4.2](#42-output-maps) and emit exactly one deterministic spelling. The result is the **bare canonical transform body** — the input-domain fields plus the explicit `output`. It does **not** carry a `kind` field (`kind` is a discriminator on *input messages* only; its omission is what makes the body a tensorstore-loadable `IndexTransform`, [section 2.1](#21-the-one-structural-difference-the-kind-discriminator)). The normalized body MUST satisfy:

- `input_rank` present.
- `input_inclusive_min` present and fully written out (no omission; per-dimension implicit/explicit flags preserved).
- The upper bound expressed as `input_exclusive_max` (whichever spelling the input used), fully written out.
- `input_labels` present (all `""` if none were given).
- `output` present and fully written out (no implicit identity; every map explicit).
- For `single_input_dimension` and `index_array` maps, `offset` and `stride` present even when equal to their defaults.
- For `index_array` maps, `index_array_bounds` present.
- For `constant` maps, only `offset` present; `stride` and `input_dimension` absent.
- `normalize` MUST be **idempotent**: `normalize(normalize(x)) == normalize(x)`.

The canonical form is intentionally verbose. Compactness is the job of the shorthands, not the canonical core.

---

## 5. Shorthands and their desugarings

Each shorthand reduces to a normalized `transform` by the rules below. Implementations MUST be observably equivalent to desugaring before acting.

### 5.1 `point`

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `kind` | `"point"` | yes | |
| `coords` | `integer[]` | yes | The `n` source coordinates of a single cell. |

Selects one cell of an n-dimensional source; the result is a 0-D scalar.

**Desugaring:** `input_rank = 0`; `input_inclusive_min = []`, `input_exclusive_max = []`, `input_labels = []` (all present and empty). `output` is `n` `constant` maps: `output[k] = { "offset": coords[k] }`.

### 5.2 `box`

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `kind` | `"box"` | yes | |
| `inclusive_min` | `bound[]` | no | Inclusive lower bound; length `n`. If omitted, defaults to all explicit `0`. |
| `exclusive_max` | `bound[]` | no † | Exclusive upper bound. |
| `inclusive_max` | `bound[]` | no † | Inclusive upper bound (`+1`). |
| `shape` | `bound[]` | no † | Size (`min + shape`). |
| `labels` | `string[]` | no | Per-dimension labels; length `n`. |

† Same upper-bound rule as [section 4.1](#41-the-transform-message): at most one of `exclusive_max`/`inclusive_max`/`shape` (else `multiple_upper_bounds`); if none, defaults to implicit `+inf`.

A `box` is exactly an `IndexDomain` (the input domain of [section 4.1](#41-the-transform-message)) paired with identity output, so it accepts the **same bound forms** as a `transform`'s input domain — including infinity sentinels and the `[n]`-bracket implicit form. (`slice`, by contrast, takes plain integers only.) The result has the **same coordinate frame** as the source ([section 2.2](#22-selected-points-keep-their-source-coordinates)).

**Desugaring:** `input_rank = n`; `input_inclusive_min` and `input_exclusive_max` are the box's bounds (the upper bound converted from whichever field was given); `input_labels` as given or all `""`. `output[k] = single_input_dimension(input_dimension = k, offset = 0, stride = 1)` — identity maps, because a pure restriction performs no rearrangement.

### 5.3 `slice`

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `kind` | `"slice"` | yes | |
| `start` | `integer[]` | yes | Inclusive start; length `n`. Plain integers only (no sentinels/brackets). |
| `stop` | `integer[]` | yes | Exclusive stop; length `n`. |
| `step` | `integer[]` | no | Length `n`; defaults to all `1`. Each element MUST be non-zero (`0` → `step_zero`). |
| `labels` | `string[]` | no | Per-dimension labels; length `n`. |

A per-dimension regular strided region following Python/NumPy slice conventions (`stop` exclusive). Mismatched array lengths → `rank_mismatch`.

**Negative `step` is reserved.** A future version will define its desugaring; until then, an implementation MUST reject a negative `step` with `negative_step_unsupported` (the conformance corpus requires this).

**Desugaring for `step[k] = s > 0`**, with `start[k] = a`, `stop[k] = b`:

- `m = max(0, ceil((b − a) / s))` — number of selected points.
- `o = floor(a / s)` — result-dimension origin (the source origin divided by the stride, preserving the source coordinate frame).
- `offset = a − s · o` (equivalently `a mod s`, in `[0, s)`) — the lattice phase, intrinsic to the selection.
- `output[k] = single_input_dimension(input_dimension = k, offset, stride = s)`.
- Input domain for dimension `k`: `[o, o + m)` (i.e. `input_inclusive_min[k] = o`, `input_exclusive_max[k] = o + m`).

The result domain is `[o, o + m)` in **source** coordinates, **not** re-based to `[0, m)` ([section 2.2](#22-selected-points-keep-their-source-coordinates)). For a 0-origin result, apply an explicit translation via `transform`.

### 5.4 `points`

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `kind` | `"points"` | yes | |
| `coords` | `integer[][]` | yes | `m` points in row-major form (each inner array is one point). All inner arrays MUST have equal length `n` (the source rank), else `rank_mismatch`. `n` may be `0` (scalar points). |

An arbitrary explicit set of `m` points; the result is a 1-D array of length `m` indexed `[0, m)`.

**Desugaring:** `input_rank = 1`; input domain `[0, m)` (`input_inclusive_min = [0]`, `input_exclusive_max = [m]`); `input_labels = [""]`. `output` is `n` `index_array` maps — the row-major coordinate list transposed to columns: `output[k] = index_array(index_array = [coords[0][k], …, coords[m−1][k]], offset = 0, stride = 1, index_array_bounds = ["-inf", "+inf"])`. When `n = 0`, `output = []`.

---

## 6. Error codes

An implementation MUST reject an invalid input with exactly one of the following reason codes, and MUST NOT produce a reason code for a valid input.

| Code | Condition |
|------|-----------|
| `invalid_json` | Not valid JSON, not a JSON object, lacks a string `kind`, or has a field whose JSON type does not match this specification (e.g. a missing required field, a boolean where an `integer` is required, a non-array bound). |
| `unknown_kind` | `kind` is a string but not one of `point`/`box`/`slice`/`points`/`transform` (including the empty string). |
| `multiple_upper_bounds` | More than one of `exclusive_max`/`inclusive_max`/`shape` (or their `input_`-prefixed forms) is present. |
| `rank_mismatch` | `input_rank` is present and inconsistent with an array length, or arrays of inconsistent lengths are provided (including ragged `points`). |
| `step_zero` | A `slice` `step` element is `0`. |
| `negative_step_unsupported` | A `slice` `step` element is negative (reserved; see [section 5.3](#53-slice)). |

---

## 7. Validation deferred in v1

For interoperability the spec describes the *intended* structure of certain fields, but v1 conformance does **not** require implementations to enforce the following. An implementation MAY accept structurally-odd `transform`s that violate these; such inputs are not covered by the conformance corpus, and behavior on them is implementation-defined:

- **`index_array` shape.** A semantically valid `index_array` is a nested integer array whose rank equals `input_rank` and is broadcast-compatible with the input domain. v1 implementations carry `index_array` verbatim and do **not** validate its rank or broadcast shape.
- **`input_dimension` range.** A `single_input_dimension` map's `input_dimension` is intended to be `< input_rank`. v1 implementations check only that it is a non-negative integer.

A future version MAY promote any of these to a required check (with an allocated reason code and corpus fixtures).

---

## 8. Conformance

An implementation is **conformant** if and only if:

1. For every success fixture in `/conformance/`, its `normalize` reproduces the fixture's `normalized` value by structural JSON equality.
2. For every error fixture, its `normalize` rejects the input with the reason code in the fixture's `error` field.

Fixtures have the forms:

```json
{ "input": <any ndsel message>, "normalized": <canonical transform body, without `kind`> }
{ "input": <any ndsel message>, "error": "<reason-code>" }
```

The `normalized` value is the bare canonical transform body of [section 4.3](#43-canonical-normalized-form) (no `kind`). Multiple independent implementations passing the same corpus is the primary evidence that this specification is unambiguous and implementable.

---

## 9. Out of scope

The following are explicitly outside the scope of ndsel v1:

- **Chunk decomposition.** ndsel models selection over the logical array index space; pushing a selection down into per-chunk selections is a consumer's concern.
- **Data values.** ndsel describes *which* points are selected and *how* they are arranged, never array contents.
- **Operation chaining.** Messages are denotational; there is no on-the-wire composition or sequencing of ndsel messages.

---

## 10. References

- **NumPy — Indexing on ndarrays.** <https://numpy.org/doc/stable/user/basics.indexing.html> — the basic-slicing and advanced-indexing semantics that ndsel represents.
- **Google tensorstore.** <https://google.github.io/tensorstore/> — the array storage/indexing library whose index model ndsel adapts.
- **tensorstore — Index space.** <https://google.github.io/tensorstore/index_space.html> — the `IndexTransform` / `IndexDomain` model that ndsel's canonical core ([section 4](#4-the-canonical-core-kind-transform)) adopts.
- **tensorstore — `IndexTransform` JSON.** <https://google.github.io/tensorstore/python/api/tensorstore.IndexTransform.__init__-json.html> — the exact JSON encoding whose field names ndsel reuses for the `transform` kind.

---

*This spec adapts the index model of Google tensorstore; see the design doc for rationale.*
