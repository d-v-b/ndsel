# ndsel — Normative Specification

**Version:** 1.0-draft.2
**Date:** 2026-07-31

*Changes since 1.0-draft (2026-06-15): negative `step` is now specified, and a reversed
slice interval is an error rather than an empty selection. Both are **breaking**; see
[section 12](#12-revision-history).*

---

## 1. Overview

`ndsel` ("n-dimensional selection") is a **JSON-serializable representation of NumPy-style n-dimensional array indexing** — the indexing written as `a[3]`, `a[2:10:2]`, or `a[[1, 4, 7]]`. It turns such an expression — normally code, evaluated in-process against a single array — into **data**: one language-neutral message that can be serialized, stored, sent across a process or language boundary, and applied to any array of matching rank.

Conceptually, every index over an n-dimensional integer grid has two parts:

1. **The input selection** — *which* points of the source grid are chosen: a contiguous block, a strided lattice, an arbitrary set of points.
2. **The output arrangement** — *how* the chosen points are laid out in the new result array.

The arrangement is a genuine choice, not a formality. The same points may be kept as an n-D sub-array that preserves their relative positions — what *basic slicing* does (`a[2:5, 1:4]` → a 3×3 block) — or **linearized** into a 1-D list — what *advanced indexing* does (`a[[2,3,4], [1,2,3]]` → the points `(2,1), (3,2), (4,3)` as a length-3 vector). NumPy folds this choice into the indexing style; ndsel states it explicitly. (In the canonical form, these two parts are the transform's **input domain** and **output maps**, [section 4](#4-the-canonical-core-kind-transform).)

This is *selection*, not a transformation of the source data: a selected point keeps its source coordinate — selecting picks out existing points, it does not renumber them into a fresh index space ([section 2.2](#22-selected-points-keep-their-source-coordinates)). What a message lays out is the *result*: how the selected points fill the new array.

The canonical representation is the `transform` kind ([section 4](#4-the-canonical-core-kind-transform)), borrowed from TensorStore's `IndexTransform`. For the common kinds (`point`, `box`, `slice`, `points`) the arrangement is the natural one — the selected points in their source order; genuine *rearrangement* (reordering axes, inserting degenerate ones) is available only through `transform`, and the shorthands never rearrange.

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

## 2. Relationship to TensorStore

ndsel **adapts the index model of Google's [tensorstore](https://google.github.io/tensorstore/)**. TensorStore's `IndexTransform` — an input domain plus a closed set of output index-map kinds — covers affine slicing, transposition, new axes, and arbitrary point selection uniformly. ndsel takes this model as its rigorous canonical core and keeps TensorStore's **exact field names** for the `transform` kind (`input_inclusive_min`, `input_exclusive_max`, `output`, `offset`, `stride`, `input_dimension`, `index_array`, `index_array_bounds`, …).

### 2.1 The one structural difference: the `kind` discriminator

ndsel is a **tagged union** of message kinds (`point`/`box`/`slice`/`points`/`transform`), so every message carries a `kind` string field. TensorStore's `IndexTransform` is a single type with no such field. This is the **only** structural difference between an ndsel `transform` and a TensorStore `IndexTransform`:

- **ndsel → TensorStore:** the normalized `transform` body ([section 4.3](#43-canonical-normalized-form)) is, field-for-field, a TensorStore `IndexTransform` *except* for the `kind` member. Because TensorStore's JSON binding is strict about unrecognized members, **strip `kind`** at the boundary, after which the body loads as an ordinary `IndexTransform`.
- **TensorStore → ndsel:** a raw `IndexTransform` has no `kind`; **add `kind: "transform"`** to present it to ndsel's `parse`.

The shorthands (`point`/`box`/`slice`/`points`) are ndsel's own; TensorStore has no JSON encoding for them. They merely desugar to ordinary `transform`s.

### 2.2 Selected points keep their source coordinates

Because a message **selects** existing points rather than transforming them, a selected point keeps the coordinate it had in the source. The result is therefore *anchored to the source frame*, not renumbered to a fresh 0-based index space: selecting `[5, 10)` keeps the domain `[5, 10)`, it is **not** re-based to `[0, 5)`. (This is also TensorStore's convention; the renumbering ndsel avoids is **NumPy's**, which re-bases sliced axes to origin 0.) Re-basing a result to origin 0 MUST be requested explicitly via a `transform`; no shorthand does it implicitly.

A direct consequence: a `box` — which selects a contiguous block and reorders nothing — desugars to **identity** output maps. The points it selects are exactly where they were, so the canonical form is a pure domain restriction with no coordinate mapping at all. Genuine *rearrangement* (reordering or inserting axes) is available, but only through the `transform` kind; the shorthands never rearrange.

### 2.3 TensorStore's JSON is a *minimal* encoding (non-normative)

The field names are shared ([section 2.1](#21-the-one-structural-difference-the-kind-discriminator)), but the two formats make opposite choices about redundancy. ndsel's normalized form ([section 4.3](#43-canonical-normalized-form)) is **maximal**: every field written out, defaults included. TensorStore's `to_json` is **minimal**: it drops anything it can re-derive. Observed against TensorStore 0.1.84:

| Dropped by `to_json` | Example |
|----------------------|---------|
| An **identity** `output` (every map `single_input_dimension(k, 0, 1)`) | the `output` member is absent entirely |
| An **all-empty** `input_labels` | `["", ""]` → member absent |
| A **redundant** `index_array_bounds` (no narrower than the hull of the array's own values) | `{"index_array": [7,3,5], "index_array_bounds": [0,9]}` → the bounds member is absent |
| A dimension whose bounds are **implicit ±inf** | a fully-implicit-infinite rank-*n* transform → `{"input_rank": n}` |

Consequences, both directions:

- **Producing:** a conformant ndsel producer emits the [section 4.3](#43-canonical-normalized-form) canonical form. A body obtained straight from TensorStore's `to_json` is a *valid ndsel message* but is **not** in canonical form; run it through `normalize` before comparing it against a canonical body.
- **Consuming:** consumers MUST accept both spellings. They are already required to — every dropped member is an omission the [section 4.1](#41-the-transform-message)/[section 4.2](#42-output-maps) defaults cover — but implementations that compare messages structurally rather than normalizing first will disagree with TensorStore over encodings that denote the same selection.

---

## 3. Conventions and value types

### 3.1 The `kind` discriminator

Every ndsel message MUST be a JSON object with a string `kind` field. Implementations MUST reject input that is not a JSON object, or that lacks a string `kind`, with reason code `invalid_json`. A `kind` value that is a string but not one of the five recognized kinds (including the empty string) MUST be rejected with `unknown_kind` ([section 6](#6-error-codes)).

### 3.2 Infinity sentinels

The strings `"-inf"` and `"+inf"` (exactly those characters, including the sign) represent negative and positive infinity in **bound** positions (domain bounds and `index_array_bounds`). Implementations MUST accept them wherever a bound is legal, and MUST NOT accept bare numeric infinity (JSON has no `Infinity`). Sentinels are **not** legal in plain-integer positions (`coords`, `start`, `stop`, `step`, `input_dimension`, `input_rank`).

The sentinels are **conceptual** infinities, not stand-in integers: an implementation MUST NOT perform arithmetic on a `"-inf"`/`"+inf"` bound. Comparison against the extended-integer order (`-inf < n < +inf`) is the only operation defined on them. Where the specification derives one bound from another — `input_inclusive_max + 1`, `inclusive_min + shape` ([section 4.1](#41-the-transform-message)) — the derivation is defined only for finite operands; an infinite operand is either an error or leaves the bound infinite, at the implementation's discretion, but never a computed finite value.

> **Why (non-normative).** Libraries that back infinity with a finite sentinel compute garbage if arithmetic reaches it. TensorStore's sentinel is `2^62 − 1`, and `{"inclusive_min": "-inf", "shape": [10]}` there yields the finite, unsaturated, nonsensical interval `[−4611686018427387903, −4611686018427387893)` — silently. A related trap at the same boundary: TensorStore's *exclusive* upper infinity is `kInfIndex + 1 = 2^62`, so a bridge mapping `"+inf"` in exclusive-max position to `kInfIndex` is off by one and produces a finite bound.

### 3.3 Implicit bounds: the `[n]`-bracket convention

A bound wrapped in a single-element JSON array — `[n]`, `["-inf"]`, or `["+inf"]` — denotes an **implicit** bound. It carries the same value as the bare form but signals that the dimension's extent is not fixed by the message; a consumer MAY substitute the actual array extent. A bare value denotes an **explicit** bound. The implicit/explicit distinction is preserved through normalization ([section 4.3](#43-canonical-normalized-form)).

### 3.4 Rank

The input rank of a `transform` is a non-negative integer. The 32-dimension ceiling is a TensorStore *implementation* constraint, not a property of this format: implementations that interoperate with TensorStore SHOULD reject ranks greater than 32, but ndsel itself imposes no upper bound.

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

### 3.7 Strict membership

A message object MUST contain only the members defined for its `kind` (the fields of its [section 4](#4-the-canonical-core-kind-transform)/[section 5](#5-shorthands-and-their-desugarings) table, plus `kind`), and an `output-map` object only the members of [section 4.2](#42-output-maps). Any other member is rejected with `unknown_field` — so a misspelled field fails loudly rather than being silently ignored. This matches the JSON Schema's `additionalProperties: false` and keeps a normalized `transform` loadable by TensorStore's strict JSON binding ([section 2.1](#21-the-one-structural-difference-the-kind-discriminator)). A future version that adds fields will do so under an explicit extension mechanism rather than by relaxing this rule.

---

## 4. The canonical core: `kind: "transform"`

### 4.1 The `transform` message

A `transform` message is a JSON object. All members except `kind` follow TensorStore's `IndexTransform` JSON encoding.

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

**Domain validity.** After the upper bound is resolved, every dimension MUST satisfy `inclusive_min ≤ exclusive_max` in the extended-integer order (`-inf < n < +inf`). An **empty** interval, where the two are equal, is valid (a zero-length dimension); an **inverted** one — `inclusive_min > exclusive_max`, including the interval produced by a negative `shape` — is rejected with `bounds_out_of_order`. This mirrors TensorStore's `IndexInterval`, whose size is non-negative.

### 4.2 Output maps

An output map is **not tagged**. Unlike a message (which carries a `kind`), a map's kind is determined by **which fields are present**. This matches TensorStore's `IndexTransform` encoding, so the canonical body stays a loadable `IndexTransform` ([section 2.1](#21-the-one-structural-difference-the-kind-discriminator)); and it is unambiguous because the three kinds are distinguished by genuinely different content — a map either consumes an input dimension, or looks up an array, or is constant — so the distinguishing field *is* the natural marker rather than a redundant tag. A map MUST NOT carry both `input_dimension` and `index_array`; one that does is rejected with `output_map_conflict`.

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

`normalize(message) → transform` MUST accept all the redundancies of [section 4.1](#41-the-transform-message)–[section 4.2](#42-output-maps) and emit exactly one deterministic spelling. The result is the **bare canonical transform body** — the input-domain fields plus the explicit `output`. It does **not** carry a `kind` field (`kind` is a discriminator on *input messages* only; its omission is what makes the body a TensorStore-loadable `IndexTransform`, [section 2.1](#21-the-one-structural-difference-the-kind-discriminator)). The normalized body MUST satisfy:

- `input_rank` present.
- `input_inclusive_min` present and fully written out (no omission; per-dimension implicit/explicit flags preserved).
- The upper bound expressed as `input_exclusive_max` (whichever spelling the input used), fully written out.
- `input_labels` present (all `""` if none were given).
- `output` present and fully written out (no implicit identity; every map explicit).
- For `single_input_dimension` and `index_array` maps, `offset` and `stride` present even when equal to their defaults.
- For `index_array` maps, `index_array_bounds` present.
- For `constant` maps, only `offset` present; `stride` and `input_dimension` absent.
- `normalize` MUST be **idempotent**: `normalize(normalize(x)) == normalize(x)`.

The canonical form is intentionally verbose. Compactness is the job of the shorthands, not the canonical core. It is also the *opposite* of TensorStore's `to_json`, which omits every re-derivable member; a body straight from TensorStore is a valid ndsel message but not a canonical one ([section 2.3](#23-tensorstores-json-is-a-minimal-encoding-non-normative)).

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
| `start` | `integer[]` | yes | Inclusive start of the traversal; length `n`. Plain integers only (no sentinels/brackets). |
| `stop` | `integer[]` | yes | Exclusive stop of the traversal; length `n`. |
| `step` | `integer[]` | no | Length `n`; defaults to all `1`. Each element MUST be non-zero (`0` → `step_zero`). MAY be negative. |
| `labels` | `string[]` | no | Per-dimension labels; length `n`. |

A per-dimension regular strided region following Python/NumPy slice conventions (`stop` exclusive). Mismatched array lengths → `rank_mismatch`.

`start` and `stop` are **required and literal**. They are source coordinates: never omitted, and never reinterpreted as counted-from-the-end when negative — a message carries no domain to resolve either convention against. The Python spellings `a[:]`, `a[::-1]`, `a[-3:]` therefore have no direct encoding; a producer resolves them against the source shape *before* emitting ([note below](#note-non-normative-lowering-a-python-slice)).

**Desugaring.** A single rule covers both signs of `step`. For dimension `k`, with `start[k] = a`, `stop[k] = b`, `step[k] = s ≠ 0`:

- **Source interval.** The traversal starts at `a` and runs toward `b`, which is excluded: the selected coordinates lie in the half-open interval `[a, b)` when `s > 0`, and in `[b + 1, a + 1)` when `s < 0`. Its **length** `L` (distinct from the rank `n` above) is
  ```
  L = b − a     (s > 0)
  L = a − b     (s < 0)
  ```
- **Validity.** `L < 0` — `b` on the far side of `a` from the direction of travel — is rejected with `bounds_out_of_order`. `L = 0` is valid and selects nothing; an empty selection is legal at **any** coordinate.
- `m = ceil(L / |s|)` — number of selected points.
- `o = trunc(a / s)` — result-dimension origin, the traversal start divided by the step with **truncation toward zero** (drop the fractional part), for **both** signs of `s`. This preserves the source coordinate frame.
- `offset = a − s · o` — the lattice phase, intrinsic to the selection. It is the truncated-division remainder of `a / s`: `|offset| < |s|`, carrying the sign of `a` (and `0` when `s` divides `a`).
- `output[k] = single_input_dimension(input_dimension = k, offset, stride = s)`.
- Input domain for dimension `k`: `[o, o + m)` (i.e. `input_inclusive_min[k] = o`, `input_exclusive_max[k] = o + m`).
- `labels[k]` carries through unchanged: a strided or reversed dimension keeps its label.

The `m` selected source coordinates are `a, a + s, a + 2s, …`; equivalently, result index `i` names source coordinate `offset + s · i`, which at `i = o` is exactly `a`. The result domain is `[o, o + m)` in **source** coordinates, **not** re-based to `[0, m)` ([section 2.2](#22-selected-points-keep-their-source-coordinates)). For a 0-origin result, apply an explicit translation via `transform`.

A reversed interval is an **error**, not an empty selection: `{"start": [9], "stop": [0], "step": [2]}` and `{"start": [5], "stop": [6], "step": [-1]}` are both rejected with `bounds_out_of_order`. The boundary is sharp — `{"start": [5], "stop": [5], "step": [-1]}` is valid and empty, `{"start": [5], "stop": [6], "step": [-1]}` is not. Selecting nothing is spelled `b = a`; anything further is a mistake about the direction of travel, and the specification says so rather than silently producing an empty result.

This matches TensorStore's strided-slice desugaring, verified against TensorStore 0.1.84 by exhaustive comparison over 32,980 cases (domains, both step signs, every in- and out-of-range bound) with zero disagreement, on results and on which inputs are rejected.

**On `trunc`.** Truncation and floor agree for every `a / s ≥ 0`; they differ **exactly** when `a / s < 0` and `s` does not divide `a` — there `trunc` rounds toward zero (`trunc(−9 / 2) = −4`, `trunc(15 / −2) = −7`) while `floor` would round down (`−5`, `−8`). With `s > 0` the divergence needs a negative `a`; with `s < 0` it is the *ordinary* case, reached by any positive `a`. ndsel follows TensorStore, so it uses `trunc` throughout.

**Worked example — reversing an axis.** In NumPy, reversing a length-20 axis is `a[::-1]`. Resolved against that axis's domain `[0, 20)` (see the lowering note below) it is `start = 19`, `stop = −1`, `step = −1`:

```json
{ "kind": "slice", "start": [19], "stop": [-1], "step": [-1] }
```

`L = a − b = 19 − (−1) = 20`, so `m = 20`; `o = trunc(19 / −1) = −19`; `offset = 19 − (−1)·(−19) = 0`. The normalized transform is domain `[−19, 1)` with the single map `single_input_dimension(input_dimension = 0, offset = 0, stride = −1)`. Reading it back: result index `i` names source coordinate `0 + (−1)·i`, so `i = −19, −18, …, 0` names source points `19, 18, …, 0` — the axis, reversed, still in source coordinates.

**A negative step normally yields a negative origin.** `o = trunc(a / s)` carries the sign of `a / s`, so for `s < 0` and a non-negative traversal start the origin is `≤ 0`: reversing a 0-origin axis puts almost the whole result domain below zero. That is the direct consequence of [section 2.2](#22-selected-points-keep-their-source-coordinates) — the result stays anchored to the source frame, and a reversing map necessarily runs the frame backwards — but it is the first construction in which a message built from a non-negative source produces negative coordinates. A consumer that requires non-negative coordinates (NumPy has no origin) re-bases explicitly; nothing in the desugaring does it implicitly.

<a id="note-non-normative-lowering-a-python-slice"></a>

> **Note (non-normative): lowering a Python slice.** A Python/NumPy slice may omit either bound, and ndsel's `slice` may not — the message has no domain to default against. A producer holding the source domain `[lo, hi)` for the dimension resolves omissions **on the side the traversal starts and stops**, in literal source coordinates:
>
> | | traversal start `a` | traversal stop `b` |
> |---|---|---|
> | `s > 0` | `start` if given, else `lo` | `stop` if given, else `hi` |
> | `s < 0` | `start` if given, else `hi − 1` | `stop` if given, else `lo − 1` |
>
> These are the rules TensorStore applies (verified, same corpus as above), and consumers that evaluate Python-style slices against a domain SHOULD apply them so that implementations agree. Note that for `s < 0` the omitted stop is `lo − 1` — one *below* the domain — which is what makes `[::-1]` select down to and including `lo`; for `lo = 0` it is the literal coordinate `−1`, **not** "the last element".
>
> Two conventions a producer must resolve itself, because neither ndsel nor TensorStore applies them: **from-the-end negative indices** (NumPy's `a[-3:]` means `a[hi−3:]`; ndsel reads `−3` as the coordinate `−3`), and **clamping** (NumPy silently clamps `a[0:1000]` to the axis; an ndsel bound outside the domain is either an error or an assertion about the domain, depending on the consumer).



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
| `unknown_field` | A message or output map contains a member not defined for its kind (see [section 3.7](#37-strict-membership)). |
| `multiple_upper_bounds` | More than one of `exclusive_max`/`inclusive_max`/`shape` (or their `input_`-prefixed forms) is present. |
| `bounds_out_of_order` | A dimension's interval is inverted: its `inclusive_min` exceeds its `exclusive_max` — including the interval produced by a negative `shape`, and the source interval of a `slice` whose `stop` lies on the far side of `start` from the direction of travel ([section 5.3](#53-slice)). An *empty* interval (`inclusive_min == exclusive_max`) is valid. |
| `output_map_conflict` | An output map carries both `input_dimension` and `index_array` ([section 4.2](#42-output-maps)). |
| `rank_mismatch` | `input_rank` is present and inconsistent with an array length, or arrays of inconsistent lengths are provided (including ragged `points`). |
| `step_zero` | A `slice` `step` element is `0`. |

The code `negative_step_unsupported` was **retired** in 1.0-draft.2, when negative `step` became specified ([section 12](#12-revision-history)). It MUST NOT be emitted; a conformant implementation has no condition that produces it.

---

## 7. Validation deferred in v1

For interoperability the spec describes the *intended* structure of certain fields, but v1 conformance does **not** require implementations to enforce the following. An implementation MAY accept structurally-odd `transform`s that violate these; such inputs are not covered by the conformance corpus, and behavior on them is implementation-defined:

- **`index_array` shape.** A semantically valid `index_array` is a nested integer array whose rank equals `input_rank` and is broadcast-compatible with the input domain. v1 implementations carry `index_array` verbatim and do **not** validate its rank or broadcast shape.
- **`input_dimension` range.** A `single_input_dimension` map's `input_dimension` is intended to be `< input_rank`. v1 implementations check only that it is a non-negative integer.

A future version MAY promote any of these to a required check (with an allocated reason code and corpus fixtures).

### 7.1 Producing an `index_array` that can be read back (non-normative)

`normalize` carries `index_array` verbatim, so an empty one survives it unchanged and round-trips — the corpus pins this. An implementation that *serializes an in-memory transform* has a further problem, and this note records it so that each one does not have to rediscover it.

A semantically valid `index_array` has rank equal to `input_rank`. JSON nested arrays cannot express every empty shape at that rank: an array is rendered by nesting one level per axis, and once the **leading** axis is the zero-length one there are no inner arrays left to describe the rest. For `input_rank = 2`, shape `(1, 0)` is `[[]]`, but shape `(0, 1)` — and every other `(0, n)` — is `[]`, which reads back as rank 1. Both the rank and the input dimension the map varies over are lost.

The way out is that such a map never needs to be emitted. An `index_array` can only be empty because an input dimension is (the rank and broadcast-compatibility rule above leaves each axis either `1` or the domain's extent, so a `0` requires a `0`), the domain is written out separately, and no output coordinate is ever looked up through a map over an empty domain. The map is therefore degenerate: it has no observable behaviour to preserve, and a `constant` map stands in for it exactly.

TensorStore takes this route. `t[ts.d[0][[]]]` yields `out[0] = 0`, emitted as `{}` — a `constant` map — and TensorStore rejects a document carrying `{"index_array": []}` at rank 2 with *"Index array for output dimension 0 has rank 1 but must have rank 2"*. Emitting the array would therefore produce a body it could not load, from a format whose canonical bodies are meant to be loadable `IndexTransform`s ([section 2.1](#21-the-one-structural-difference-the-kind-discriminator)).

So: an implementation SHOULD collapse an `index_array` map with no elements to `constant`. The same applies for the same reason to a map whose `index_array` has exactly one element, which selects one coordinate regardless of input; there the collapse is a simplification rather than a necessity, since that shape does round-trip.

This is guidance for producers, not a conformance requirement: ndsel conformance is defined over `normalize` ([section 8](#8-conformance)), which is given a message and never has an in-memory transform to serialize.

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

## 10. License

This specification is dedicated to the public domain under [CC0 1.0 Universal](../LICENSE-CC0). You may copy, modify, and implement it — in whole or in part, for any purpose, without attribution or permission. The reference implementations (`rust/`, `python/`, `typescript/`) are separately licensed under `MIT OR Apache-2.0`.

---

## 11. References

- **NumPy — Indexing on ndarrays.** <https://numpy.org/doc/stable/user/basics.indexing.html> — the basic-slicing and advanced-indexing semantics that ndsel represents.
- **TensorStore.** <https://google.github.io/tensorstore/> — Google's array storage/indexing library whose index model ndsel adapts.
- **TensorStore — Index space.** <https://google.github.io/tensorstore/index_space.html> — the `IndexTransform` / `IndexDomain` model that ndsel's canonical core ([section 4](#4-the-canonical-core-kind-transform)) adopts.
- **TensorStore — `IndexTransform` JSON.** <https://google.github.io/tensorstore/python/api/tensorstore.IndexTransform.__init__-json.html> — the exact JSON encoding whose field names ndsel reuses for the `transform` kind.

---

## 12. Revision history

### 1.0-draft.2 — 2026-07-31

Two **breaking** changes to `slice` ([section 5.3](#53-slice)), both adopting behaviour verified
against TensorStore 0.1.84 over an exhaustive 32,980-case corpus:

1. **Negative `step` is specified.** 1.0-draft reserved it and required implementations to
   reject it with `negative_step_unsupported`; the conformance corpus contained a fixture
   demanding that rejection. A negative `step` is now valid, and desugars by the same rule as
   a positive one. The reason code `negative_step_unsupported` is **retired**
   ([section 6](#6-error-codes)) and its corpus fixture is replaced by success fixtures.
   An implementation that still rejects negative `step` is no longer conformant.
2. **A reversed slice interval is an error.** 1.0-draft derived `m = max(0, ceil((b − a) / s))`,
   silently clamping a reversed interval to an empty selection; the count is now
   `m = ceil(L / |s|)` over a signed interval length `L`, and `L < 0` is rejected with
   `bounds_out_of_order`. Inputs that previously normalized to an empty domain — e.g.
   `{"start": [9], "stop": [0], "step": [2]}` — are now rejected. `L = 0` remains valid.

Non-breaking in the same revision: `o = trunc(a / s)` is stated for both signs of `s`
(1.0-draft already said `trunc`, but only defined `s > 0`); the `offset` range is restated as
the truncated-division remainder, since the old `(−s, s)` phrasing assumed `s > 0`; a
non-normative note records how a Python slice with omitted bounds lowers to a `slice` message
([section 5.3](#53-slice)); a non-normative note records that TensorStore's JSON is a minimal
encoding while ndsel's canonical form is maximal ([section 2.3](#23-tensorstores-json-is-a-minimal-encoding-non-normative));
and infinity sentinels are stated to be non-arithmetic ([section 3.2](#32-infinity-sentinels)).

### 1.0-draft — 2026-06-15

Initial draft. (Amended in place before 1.0-draft.2 to specify the strided-slice origin as
`trunc(a / s)` rather than `floor(a / s)`, matching TensorStore.)

---

*This spec adapts the index model of Google's TensorStore; see the design doc for rationale.*
