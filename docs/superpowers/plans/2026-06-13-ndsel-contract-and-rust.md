# ndsel Contract + Rust Reference — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce the ndsel contract (normative spec, JSON Schema, conformance corpus) plus a complete, corpus-verified Rust reference implementation of `normalize`.

**Architecture:** A single Rust library crate models messages as a `kind`-tagged enum and reduces every variant to a canonical `Transform` via `normalize`. The conformance corpus (JSON fixtures: `input` → `normalized` | `error`) is authored from the spec and executed by a Rust integration test that also validates each fixture against the JSON Schema. The corpus is the language-agnostic contract that Plans 2 (Python) and 3 (TypeScript) will satisfy.

**Tech Stack:** Rust (edition 2024), `serde` + `serde_json` (de/serialization), `jsonschema` (schema validation in the corpus runner). Reference design: `docs/superpowers/specs/2026-06-13-ndsel-spec-design.md`.

---

## Scope of this plan

- **In:** normative spec prose; JSON Schema (syntax validation); Rust types for all five `kind`s; `normalize` for `point`, `box`, `slice` (positive step), `points`, and `transform`; error handling for the cases below; the conformance corpus + Rust runner.
- **Deferred (tracked in spec §12):**
  - **Negative `step`** — rejected in this plan with error code `negative_step_unsupported`; positive step fully implemented.
  - **Deep `index_array` validation** — nested index arrays in a `transform` are carried faithfully as raw JSON and round-tripped, but broadcast/rank checks beyond "rectangular array of integers" are out of scope. The `points` shorthand only ever produces rank-1 index arrays.
- **Error cases implemented this plan:** `step = 0` (`step_zero`); negative step (`negative_step_unsupported`); more than one upper-bound spelling in a `box`/`transform` (`multiple_upper_bounds`); array-length disagreement with `input_rank` (`rank_mismatch`); unknown `kind` (`unknown_kind`).

## File structure

```
spec/ndsel.md                         normative spec (prose, derived from the design doc)
schema/ndsel.schema.json              JSON Schema (draft 2020-12), syntax only
conformance/*.json                   fixtures: { input, normalized } or { input, error }
conformance/README.md                fixture format contract
rust/ndsel/Cargo.toml                 lib crate + deps
rust/ndsel/src/lib.rs                 public API: Message, normalize(); module wiring
rust/ndsel/src/error.rs               NdsqError + reason codes
rust/ndsel/src/value.rs               IndexValue (i64 / ±inf) + ImplicitValue (bracket convention)
rust/ndsel/src/domain.rs             Domain (rank, inclusive_min, exclusive_max, labels) + upper-bound normalization
rust/ndsel/src/output.rs              OutputMap (constant / single_input_dimension / index_array)
rust/ndsel/src/transform.rs          Transform struct + canonicalization
rust/ndsel/src/shorthand.rs          desugar point/box/slice/points -> Transform
rust/ndsel/tests/conformance.rs      corpus runner (schema-validate + normalize + compare)
```

Each Rust source file has one responsibility. `value.rs` owns the two fiddly serde conventions (inf sentinels, `[n]` brackets) so nothing else has to.

---

## Task 1: Normative spec prose

**Files:**
- Create: `spec/ndsel.md`

- [ ] **Step 1: Write the normative spec**

Create `spec/ndsel.md` as the clean, normative version of the design doc. Copy the substance of these sections from `docs/superpowers/specs/2026-06-13-ndsel-spec-design.md`, rewritten as a specification (imperative "MUST/MAY", no "decisions log" or "open items meta"):

- **Overview** (design §1) — purpose; subset + arrangement; shorthand ladder.
- **Relationship to tensorstore** (design §2) — adaptation; the two departures (`kind` field; World A).
- **Conventions** — `kind` discriminator; `-inf`/`+inf` string sentinels; `[n]`-bracket = implicit bound; ranks in `[0, 32]`.
- **Canonical core `transform`** (design §5.1, §5.2) — accepted form, the three output-map kinds, canonical form (upper bound as `exclusive_max`, explicit `output`, explicit defaults).
- **Shorthands** (design §6) — `point`, `box`, `slice`, `points`, each with its desugaring. Include the full positive-`step` slice formula verbatim from design §6.3. State negative `step` is reserved (implementations MAY reject it until specified).
- **Errors** — the reason codes listed in "Scope of this plan".
- **Conformance** — an implementation is conformant iff its `normalize` reproduces every corpus `normalized` (structural JSON equality) and rejects every corpus `error` fixture with the stated code.
- **Out of scope** (design §9).

End with a line: *"This spec adapts the index model of Google tensorstore; see the design doc for rationale."*

- [ ] **Step 2: Commit**

```bash
git add spec/ndsel.md
git commit -m "docs: add normative ndsel spec prose"
```

---

## Task 2: Rust crate skeleton (lib, deps, module stubs)

**Files:**
- Modify: `rust/ndsel/Cargo.toml`
- Create: `rust/ndsel/src/lib.rs`
- Delete: `rust/ndsel/src/main.rs`
- Create (empty module stubs): `rust/ndsel/src/{error,value,domain,output,transform,shorthand}.rs`

- [ ] **Step 1: Convert to a library crate with dependencies**

Replace `rust/ndsel/Cargo.toml` with:

```toml
[package]
name = "ndsel"
version = "0.1.0"
edition = "2024"

[lib]
path = "src/lib.rs"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
jsonschema = "0.26"
```

- [ ] **Step 2: Remove the binary, create lib.rs with module declarations only**

Delete `rust/ndsel/src/main.rs`. Create `rust/ndsel/src/lib.rs` with ONLY the module declarations. The public API (`Message`, `normalize`, `parse`, and re-exports) is added in Task 9, once every module exists — so Tasks 3–8 each compile and test independently:

```rust
//! ndsel — JSON-serializable n-dimensional spatial queries.
//!
//! A `Message` is a `kind`-discriminated union; `normalize` reduces any message
//! to a canonical `Transform`. Adapts the index model of Google tensorstore.
//!
//! The public API is wired up in `shorthand` (Task 9) once every module exists.

mod domain;
mod error;
mod output;
mod shorthand;
mod transform;
mod value;
```

Then create six empty module stub files so the crate compiles — each of `error.rs`, `value.rs`, `domain.rs`, `output.rs`, `transform.rs`, `shorthand.rs` containing a single line:

```rust
// filled in a later task
```

Empty modules compile, so the crate builds now (it has no public API yet).

- [ ] **Step 3: Verify it builds**

Run: `cargo build -p ndsel 2>&1 | tail -5`
Expected: clean build.

- [ ] **Step 4: Commit**

```bash
git add rust/ndsel/Cargo.toml rust/ndsel/src/
git rm rust/ndsel/src/main.rs
git commit -m "chore(rust): convert ndsel to a library crate with serde"
```

---

## Task 3: `error.rs` — error type and reason codes

**Files:**
- Create: `rust/ndsel/src/error.rs`
- Test: inline `#[cfg(test)]` in `error.rs`

- [ ] **Step 1: Write the failing test**

Put this at the bottom of `rust/ndsel/src/error.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reason_code_strings_are_stable() {
        assert_eq!(Reason::StepZero.code(), "step_zero");
        assert_eq!(Reason::RankMismatch.code(), "rank_mismatch");
        assert_eq!(Reason::MultipleUpperBounds.code(), "multiple_upper_bounds");
        assert_eq!(Reason::NegativeStepUnsupported.code(), "negative_step_unsupported");
        assert_eq!(Reason::UnknownKind.code(), "unknown_kind");
        assert_eq!(Reason::InvalidJson.code(), "invalid_json");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ndsel --lib error 2>&1 | tail -20`
Expected: FAIL — `Reason` not found / does not compile.

- [ ] **Step 3: Write the implementation**

Put this ABOVE the test module in `rust/ndsel/src/error.rs`:

```rust
use std::fmt;

/// Stable machine-readable reason codes for rejected messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    StepZero,
    NegativeStepUnsupported,
    MultipleUpperBounds,
    RankMismatch,
    UnknownKind,
    InvalidJson,
}

impl Reason {
    /// The stable string used in corpus error fixtures.
    pub fn code(self) -> &'static str {
        match self {
            Reason::StepZero => "step_zero",
            Reason::NegativeStepUnsupported => "negative_step_unsupported",
            Reason::MultipleUpperBounds => "multiple_upper_bounds",
            Reason::RankMismatch => "rank_mismatch",
            Reason::UnknownKind => "unknown_kind",
            Reason::InvalidJson => "invalid_json",
        }
    }
}

/// An error produced while parsing or normalizing a message.
#[derive(Debug, Clone)]
pub struct NdsqError {
    pub reason: Reason,
    pub detail: String,
}

impl NdsqError {
    pub fn new(reason: Reason, detail: impl Into<String>) -> Self {
        NdsqError { reason, detail: detail.into() }
    }

    pub fn from_serde(err: serde_json::Error) -> Self {
        // serde's untagged/`kind` failures and type errors all collapse here.
        NdsqError::new(Reason::InvalidJson, err.to_string())
    }
}

impl fmt::Display for NdsqError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.reason.code(), self.detail)
    }
}

impl std::error::Error for NdsqError {}
```

Note: `Reason::UnknownKind` is consumed by `parse` (wired in Task 9). It exists as a variant now.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p ndsel --lib error 2>&1 | tail -20`
Expected: the `error::tests` test PASSES. The other modules are still empty stubs, which compile fine, so the crate builds and this test runs in isolation.

- [ ] **Step 5: Commit**

```bash
git add rust/ndsel/src/error.rs
git commit -m "feat(rust): error type and stable reason codes"
```

---

## Task 4: `value.rs` — IndexValue (±inf sentinels)

**Files:**
- Create/replace: `rust/ndsel/src/value.rs` (IndexValue portion)
- Test: inline in `value.rs`

- [ ] **Step 1: Write the failing test**

Add to `rust/ndsel/src/value.rs`:

```rust
#[cfg(test)]
mod index_value_tests {
    use super::*;

    fn round(json: &str) -> String {
        let v: IndexValue = serde_json::from_str(json).unwrap();
        serde_json::to_string(&v).unwrap()
    }

    #[test]
    fn finite_integer() {
        assert_eq!(round("7"), "7");
        assert!(matches!(serde_json::from_str::<IndexValue>("7").unwrap(), IndexValue::Finite(7)));
    }

    #[test]
    fn negative_infinity() {
        assert_eq!(round("\"-inf\""), "\"-inf\"");
        assert!(matches!(serde_json::from_str::<IndexValue>("\"-inf\"").unwrap(), IndexValue::NegInf));
    }

    #[test]
    fn positive_infinity() {
        assert_eq!(round("\"+inf\""), "\"+inf\"");
        assert!(matches!(serde_json::from_str::<IndexValue>("\"+inf\"").unwrap(), IndexValue::PosInf));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ndsel --lib index_value 2>&1 | tail -20`
Expected: FAIL — `IndexValue` not defined.

- [ ] **Step 3: Write the implementation**

Add ABOVE the test module in `rust/ndsel/src/value.rs`:

```rust
use serde::de::{self, Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};
use serde_json::Value;

/// A single index coordinate: a finite i64, or unbounded -inf / +inf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexValue {
    Finite(i64),
    NegInf,
    PosInf,
}

impl Serialize for IndexValue {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            IndexValue::Finite(n) => s.serialize_i64(*n),
            IndexValue::NegInf => s.serialize_str("-inf"),
            IndexValue::PosInf => s.serialize_str("+inf"),
        }
    }
}

impl<'de> Deserialize<'de> for IndexValue {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        match Value::deserialize(d)? {
            Value::Number(n) => n
                .as_i64()
                .map(IndexValue::Finite)
                .ok_or_else(|| de::Error::custom("index value must be an integer")),
            Value::String(s) if s == "-inf" => Ok(IndexValue::NegInf),
            Value::String(s) if s == "+inf" => Ok(IndexValue::PosInf),
            other => Err(de::Error::custom(format!("invalid index value: {other}"))),
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p ndsel --lib index_value 2>&1 | tail -20`
Expected: 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/ndsel/src/value.rs
git commit -m "feat(rust): IndexValue with +/-inf sentinels"
```

---

## Task 5: `value.rs` — ImplicitValue (the `[n]`-bracket convention)

**Files:**
- Modify: `rust/ndsel/src/value.rs`
- Test: inline in `value.rs`

- [ ] **Step 1: Write the failing test**

Add a second test module to `rust/ndsel/src/value.rs`:

```rust
#[cfg(test)]
mod implicit_value_tests {
    use super::*;

    fn parse(json: &str) -> ImplicitValue {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn bare_value_is_explicit() {
        let v = parse("7");
        assert_eq!(v.value, IndexValue::Finite(7));
        assert!(!v.implicit);
    }

    #[test]
    fn bracketed_value_is_implicit() {
        let v = parse("[7]");
        assert_eq!(v.value, IndexValue::Finite(7));
        assert!(v.implicit);
    }

    #[test]
    fn bracketed_inf_is_implicit() {
        let v = parse("[\"-inf\"]");
        assert_eq!(v.value, IndexValue::NegInf);
        assert!(v.implicit);
    }

    #[test]
    fn explicit_serializes_bare_implicit_serializes_bracketed() {
        assert_eq!(serde_json::to_string(&parse("7")).unwrap(), "7");
        assert_eq!(serde_json::to_string(&parse("[7]")).unwrap(), "[7]");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ndsel --lib implicit_value 2>&1 | tail -20`
Expected: FAIL — `ImplicitValue` not defined.

- [ ] **Step 3: Write the implementation**

Add to `rust/ndsel/src/value.rs` (above the implicit test module):

```rust
/// An index bound with an explicit/implicit flag.
/// JSON: a bare value is explicit; the same value wrapped in a 1-element
/// array is implicit (`7` vs `[7]`, `"-inf"` vs `["-inf"]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImplicitValue {
    pub value: IndexValue,
    pub implicit: bool,
}

impl ImplicitValue {
    pub fn explicit(value: IndexValue) -> Self {
        ImplicitValue { value, implicit: false }
    }
}

impl Serialize for ImplicitValue {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.implicit {
            // serialize as a 1-element array
            use serde::ser::SerializeSeq;
            let mut seq = s.serialize_seq(Some(1))?;
            seq.serialize_element(&self.value)?;
            seq.end()
        } else {
            self.value.serialize(s)
        }
    }
}

impl<'de> Deserialize<'de> for ImplicitValue {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        match v {
            Value::Array(items) => {
                if items.len() != 1 {
                    return Err(de::Error::custom("implicit bound must be a 1-element array"));
                }
                let inner: IndexValue = serde_json::from_value(items.into_iter().next().unwrap())
                    .map_err(de::Error::custom)?;
                Ok(ImplicitValue { value: inner, implicit: true })
            }
            other => {
                let inner: IndexValue = serde_json::from_value(other).map_err(de::Error::custom)?;
                Ok(ImplicitValue { value: inner, implicit: false })
            }
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p ndsel --lib implicit_value 2>&1 | tail -20`
Expected: 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/ndsel/src/value.rs
git commit -m "feat(rust): ImplicitValue and the [n]-bracket implicit-bound convention"
```

---

## Task 6: `domain.rs` — Domain and upper-bound normalization

**Files:**
- Create: `rust/ndsel/src/domain.rs`
- Test: inline in `domain.rs`

The `Domain` accepts any one of `exclusive_max`/`inclusive_max`/`shape`, plus `inclusive_min` (defaulting to all-0 explicit when only `shape` is present), and canonicalizes to `inclusive_min` + `exclusive_max`.

- [ ] **Step 1: Write the failing test**

Create `rust/ndsel/src/domain.rs` with this test at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::{ImplicitValue, IndexValue};

    fn fin(n: i64) -> ImplicitValue { ImplicitValue::explicit(IndexValue::Finite(n)) }

    #[test]
    fn shape_only_defaults_min_to_zero() {
        let raw: RawDomain = serde_json::from_str(r#"{ "shape": [3, 4] }"#).unwrap();
        let d = raw.into_domain().unwrap();
        assert_eq!(d.rank, 2);
        assert_eq!(d.inclusive_min, vec![fin(0), fin(0)]);
        assert_eq!(d.exclusive_max, vec![fin(3), fin(4)]);
    }

    #[test]
    fn inclusive_max_is_converted_to_exclusive() {
        let raw: RawDomain =
            serde_json::from_str(r#"{ "inclusive_min": [0], "inclusive_max": [9] }"#).unwrap();
        let d = raw.into_domain().unwrap();
        assert_eq!(d.exclusive_max, vec![fin(10)]);
    }

    #[test]
    fn multiple_upper_bounds_is_error() {
        let raw: RawDomain =
            serde_json::from_str(r#"{ "shape": [3], "exclusive_max": [3] }"#).unwrap();
        assert_eq!(raw.into_domain().unwrap_err().reason, crate::error::Reason::MultipleUpperBounds);
    }

    #[test]
    fn length_disagreement_is_rank_mismatch() {
        let raw: RawDomain =
            serde_json::from_str(r#"{ "inclusive_min": [0, 0], "shape": [3] }"#).unwrap();
        assert_eq!(raw.into_domain().unwrap_err().reason, crate::error::Reason::RankMismatch);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ndsel --lib domain 2>&1 | tail -20`
Expected: FAIL — `RawDomain` / `Domain` not defined.

- [ ] **Step 3: Write the implementation**

Add ABOVE the test module in `rust/ndsel/src/domain.rs`:

```rust
use serde::{Deserialize, Serialize};

use crate::error::{NdsqError, Reason};
use crate::value::{ImplicitValue, IndexValue};

/// The canonical input domain: per-dimension [inclusive_min, exclusive_max) + labels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Domain {
    pub rank: usize,
    pub inclusive_min: Vec<ImplicitValue>,
    pub exclusive_max: Vec<ImplicitValue>,
    pub labels: Vec<String>,
}

/// As-received domain fields, before canonicalization.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RawDomain {
    #[serde(default)]
    pub rank: Option<usize>,
    #[serde(default)]
    pub inclusive_min: Option<Vec<ImplicitValue>>,
    #[serde(default)]
    pub exclusive_max: Option<Vec<ImplicitValue>>,
    #[serde(default)]
    pub inclusive_max: Option<Vec<ImplicitValue>>,
    #[serde(default)]
    pub shape: Option<Vec<ImplicitValue>>,
    #[serde(default)]
    pub labels: Option<Vec<String>>,
}

impl RawDomain {
    pub fn into_domain(self) -> Result<Domain, NdsqError> {
        // Exactly one upper-bound spelling.
        let upper_count = self.exclusive_max.is_some() as u8
            + self.inclusive_max.is_some() as u8
            + self.shape.is_some() as u8;
        if upper_count > 1 {
            return Err(NdsqError::new(
                Reason::MultipleUpperBounds,
                "specify only one of exclusive_max / inclusive_max / shape",
            ));
        }

        // Determine rank from whichever arrays are present.
        let mut rank: Option<usize> = self.rank;
        let mut check = |len: usize, rank: &mut Option<usize>| -> Result<(), NdsqError> {
            match rank {
                Some(r) if *r != len => Err(NdsqError::new(
                    Reason::RankMismatch,
                    format!("array length {len} disagrees with rank {r}"),
                )),
                _ => {
                    *rank = Some(len);
                    Ok(())
                }
            }
        };
        if let Some(v) = &self.inclusive_min { check(v.len(), &mut rank)?; }
        if let Some(v) = &self.exclusive_max { check(v.len(), &mut rank)?; }
        if let Some(v) = &self.inclusive_max { check(v.len(), &mut rank)?; }
        if let Some(v) = &self.shape { check(v.len(), &mut rank)?; }
        if let Some(v) = &self.labels { check(v.len(), &mut rank)?; }
        let rank = rank.unwrap_or(0);

        // inclusive_min: default to explicit 0 per dimension.
        let inclusive_min = self.inclusive_min.unwrap_or_else(|| {
            vec![ImplicitValue::explicit(IndexValue::Finite(0)); rank]
        });

        // Canonicalize the upper bound to exclusive_max.
        let exclusive_max = if let Some(em) = self.exclusive_max {
            em
        } else if let Some(im) = self.inclusive_max {
            im.into_iter().map(bump_inclusive_to_exclusive).collect()
        } else if let Some(shape) = self.shape {
            inclusive_min
                .iter()
                .zip(shape.iter())
                .map(|(lo, sz)| add_shape(*lo, *sz))
                .collect()
        } else {
            // No upper bound given: unbounded implicit +inf.
            vec![ImplicitValue { value: IndexValue::PosInf, implicit: true }; rank]
        };

        let labels = self.labels.unwrap_or_else(|| vec![String::new(); rank]);

        Ok(Domain { rank, inclusive_min, exclusive_max, labels })
    }
}

/// inclusive_max n -> exclusive_max n+1 (preserving implicit flag; +inf stays +inf).
fn bump_inclusive_to_exclusive(v: ImplicitValue) -> ImplicitValue {
    let value = match v.value {
        IndexValue::Finite(n) => IndexValue::Finite(n + 1),
        other => other,
    };
    ImplicitValue { value, implicit: v.implicit }
}

/// exclusive_max = inclusive_min + shape (preserving min's implicit flag; inf saturates).
fn add_shape(lo: ImplicitValue, sz: ImplicitValue) -> ImplicitValue {
    let value = match (lo.value, sz.value) {
        (IndexValue::Finite(a), IndexValue::Finite(s)) => IndexValue::Finite(a + s),
        (IndexValue::PosInf, _) | (_, IndexValue::PosInf) => IndexValue::PosInf,
        (IndexValue::NegInf, _) | (_, IndexValue::NegInf) => IndexValue::NegInf,
    };
    ImplicitValue { value, implicit: lo.implicit }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p ndsel --lib domain 2>&1 | tail -20`
Expected: 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/ndsel/src/domain.rs
git commit -m "feat(rust): Domain with upper-bound canonicalization to exclusive_max"
```

---

## Task 7: `output.rs` — OutputMap (three kinds, default filling)

**Files:**
- Create: `rust/ndsel/src/output.rs`
- Test: inline in `output.rs`

`OutputMap` carries raw `index_array` JSON faithfully (deferred deep validation, see Scope). Canonical form always writes explicit `offset`/`stride`, and `index_array_bounds` for index-array maps.

- [ ] **Step 1: Write the failing test**

Create `rust/ndsel/src/output.rs` with this test at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_map_carries_only_offset() {
        let m: RawOutputMap = serde_json::from_str(r#"{ "offset": 3 }"#).unwrap();
        let canon = m.canonicalize();
        assert_eq!(canon, OutputMap::Constant { offset: 3 });
        assert_eq!(serde_json::to_value(&canon).unwrap(), serde_json::json!({ "offset": 3 }));
    }

    #[test]
    fn single_input_dimension_fills_defaults() {
        let m: RawOutputMap = serde_json::from_str(r#"{ "input_dimension": 2 }"#).unwrap();
        let canon = m.canonicalize();
        assert_eq!(
            serde_json::to_value(&canon).unwrap(),
            serde_json::json!({ "offset": 0, "stride": 1, "input_dimension": 2 })
        );
    }

    #[test]
    fn index_array_fills_offset_stride_and_bounds() {
        let m: RawOutputMap =
            serde_json::from_str(r#"{ "index_array": [1, 2, 3] }"#).unwrap();
        let canon = m.canonicalize();
        assert_eq!(
            serde_json::to_value(&canon).unwrap(),
            serde_json::json!({
                "offset": 0, "stride": 1,
                "index_array": [1, 2, 3],
                "index_array_bounds": ["-inf", "+inf"]
            })
        );
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ndsel --lib output 2>&1 | tail -20`
Expected: FAIL — types not defined.

- [ ] **Step 3: Write the implementation**

Add ABOVE the test module in `rust/ndsel/src/output.rs`:

```rust
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde::Deserialize;
use serde_json::Value;

use crate::value::IndexValue;

/// One output index map (canonical form).
#[derive(Debug, Clone, PartialEq)]
pub enum OutputMap {
    Constant { offset: i64 },
    SingleInputDimension { offset: i64, stride: i64, input_dimension: usize },
    IndexArray { offset: i64, stride: i64, index_array: Value, bounds: (IndexValue, IndexValue) },
}

/// As-received output map before default filling / kind discrimination.
#[derive(Debug, Clone, Deserialize)]
pub struct RawOutputMap {
    #[serde(default)]
    pub offset: Option<i64>,
    #[serde(default)]
    pub stride: Option<i64>,
    #[serde(default)]
    pub input_dimension: Option<usize>,
    #[serde(default)]
    pub index_array: Option<Value>,
    #[serde(default)]
    pub index_array_bounds: Option<(IndexValue, IndexValue)>,
}

impl RawOutputMap {
    pub fn canonicalize(self) -> OutputMap {
        let offset = self.offset.unwrap_or(0);
        let stride = self.stride.unwrap_or(1);
        if let Some(arr) = self.index_array {
            let bounds = self
                .index_array_bounds
                .unwrap_or((IndexValue::NegInf, IndexValue::PosInf));
            OutputMap::IndexArray { offset, stride, index_array: arr, bounds }
        } else if let Some(dim) = self.input_dimension {
            OutputMap::SingleInputDimension { offset, stride, input_dimension: dim }
        } else {
            OutputMap::Constant { offset }
        }
    }
}

impl Serialize for OutputMap {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            OutputMap::Constant { offset } => {
                let mut st = s.serialize_struct("OutputMap", 1)?;
                st.serialize_field("offset", offset)?;
                st.end()
            }
            OutputMap::SingleInputDimension { offset, stride, input_dimension } => {
                let mut st = s.serialize_struct("OutputMap", 3)?;
                st.serialize_field("offset", offset)?;
                st.serialize_field("stride", stride)?;
                st.serialize_field("input_dimension", input_dimension)?;
                st.end()
            }
            OutputMap::IndexArray { offset, stride, index_array, bounds } => {
                let mut st = s.serialize_struct("OutputMap", 4)?;
                st.serialize_field("offset", offset)?;
                st.serialize_field("stride", stride)?;
                st.serialize_field("index_array", index_array)?;
                st.serialize_field("index_array_bounds", &[bounds.0, bounds.1])?;
                st.end()
            }
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p ndsel --lib output 2>&1 | tail -20`
Expected: 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/ndsel/src/output.rs
git commit -m "feat(rust): OutputMap with three kinds and default filling"
```

---

## Task 8: `transform.rs` — Transform struct and canonicalize

**Files:**
- Create: `rust/ndsel/src/transform.rs`
- Test: inline in `transform.rs`

`Transform` deserializes from the `transform` message body, then `canonicalize()` produces the canonical form: domain via `RawDomain`, explicit `output` (identity if omitted), default-filled maps.

- [ ] **Step 1: Write the failing test**

Create `rust/ndsel/src/transform.rs` with this test at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omitted_output_becomes_explicit_identity() {
        // 2-D box-like domain, no output -> identity single_input_dimension maps.
        let raw: Transform = serde_json::from_str(
            r#"{ "input_inclusive_min": [0, 0], "input_exclusive_max": [3, 4] }"#,
        ).unwrap();
        let canon = raw.canonicalize().unwrap();
        let v = serde_json::to_value(&canon).unwrap();
        assert_eq!(v["input_inclusive_min"], serde_json::json!([0, 0]));
        assert_eq!(v["input_exclusive_max"], serde_json::json!([3, 4]));
        assert_eq!(
            v["output"],
            serde_json::json!([
                { "offset": 0, "stride": 1, "input_dimension": 0 },
                { "offset": 0, "stride": 1, "input_dimension": 1 }
            ])
        );
    }

    #[test]
    fn canonicalize_is_idempotent() {
        let raw: Transform = serde_json::from_str(
            r#"{ "input_inclusive_min": [0], "input_shape": [5] }"#,
        ).unwrap();
        let once = raw.canonicalize().unwrap();
        let twice: Transform =
            serde_json::from_value(serde_json::to_value(&once).unwrap()).unwrap();
        let twice = twice.canonicalize().unwrap();
        assert_eq!(
            serde_json::to_value(&once).unwrap(),
            serde_json::to_value(&twice).unwrap()
        );
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ndsel --lib transform 2>&1 | tail -20`
Expected: FAIL — `Transform` not defined.

- [ ] **Step 3: Write the implementation**

Add ABOVE the test module in `rust/ndsel/src/transform.rs`:

```rust
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde::Deserialize;

use crate::domain::{Domain, RawDomain};
use crate::error::NdsqError;
use crate::output::{OutputMap, RawOutputMap};
use crate::value::ImplicitValue;

/// The canonical core. Deserializes from the `transform` message body using the
/// `input_`-prefixed field names (distinct from the un-prefixed `box` names).
/// The `kind` field present in the message is ignored (no `deny_unknown_fields`).
#[derive(Debug, Clone, Deserialize)]
pub struct Transform {
    #[serde(rename = "input_rank", default)]
    rank: Option<usize>,
    #[serde(rename = "input_inclusive_min", default)]
    inclusive_min: Option<Vec<ImplicitValue>>,
    #[serde(rename = "input_exclusive_max", default)]
    exclusive_max: Option<Vec<ImplicitValue>>,
    #[serde(rename = "input_inclusive_max", default)]
    inclusive_max: Option<Vec<ImplicitValue>>,
    #[serde(rename = "input_shape", default)]
    shape: Option<Vec<ImplicitValue>>,
    #[serde(rename = "input_labels", default)]
    labels: Option<Vec<String>>,
    #[serde(default)]
    output: Option<Vec<RawOutputMap>>,

    // Populated by canonicalize(); never deserialized.
    #[serde(skip)]
    canon: Option<CanonTransform>,
}

#[derive(Debug, Clone)]
struct CanonTransform {
    domain: Domain,
    output: Vec<OutputMap>,
}

impl Transform {
    /// Produce the canonical form: domain normalized, output explicit (identity
    /// if omitted), all maps default-filled.
    pub fn canonicalize(self) -> Result<Transform, NdsqError> {
        // Reuse the shared domain canonicalization by mapping the input_-prefixed
        // fields onto a RawDomain.
        let raw_domain = RawDomain {
            rank: self.rank,
            inclusive_min: self.inclusive_min,
            exclusive_max: self.exclusive_max,
            inclusive_max: self.inclusive_max,
            shape: self.shape,
            labels: self.labels,
        };
        let domain = raw_domain.into_domain()?;
        let output = match self.output {
            Some(maps) => maps.into_iter().map(RawOutputMap::canonicalize).collect(),
            None => identity_output(&domain),
        };
        Ok(from_parts(domain, output))
    }
}

/// Identity: one single_input_dimension map per input dimension.
fn identity_output(domain: &Domain) -> Vec<OutputMap> {
    (0..domain.rank)
        .map(|k| OutputMap::SingleInputDimension { offset: 0, stride: 1, input_dimension: k })
        .collect()
}

impl Serialize for Transform {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // A Transform must be canonicalized before serialization.
        let canon = self
            .canon
            .as_ref()
            .expect("serialize a Transform only after canonicalize()");
        let mut st = s.serialize_struct("Transform", 5)?;
        st.serialize_field("input_rank", &canon.domain.rank)?;
        st.serialize_field("input_inclusive_min", &canon.domain.inclusive_min)?;
        st.serialize_field("input_exclusive_max", &canon.domain.exclusive_max)?;
        st.serialize_field("input_labels", &canon.domain.labels)?;
        st.serialize_field("output", &canon.output)?;
        st.end()
    }
}

/// Constructor used by the shorthand desugarers to build a canonical transform directly.
pub(crate) fn from_parts(domain: Domain, output: Vec<OutputMap>) -> Transform {
    Transform {
        rank: None,
        inclusive_min: None,
        exclusive_max: None,
        inclusive_max: None,
        shape: None,
        labels: None,
        output: None,
        canon: Some(CanonTransform { domain, output }),
    }
}
```

Note for the worker: `from_parts` lets `shorthand.rs` (Task 9) build canonical transforms without round-tripping through `RawDomain`. The `canon` field is the source of truth for serialization; `canonicalize()` must be called on parsed transforms before `normalize` returns them.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p ndsel --lib transform 2>&1 | tail -20`
Expected: 2 tests PASS.

- [ ] **Step 5: Run the module suite so far**

Run: `cargo test -p ndsel --lib 2>&1 | tail -20`
Expected: all tests from Tasks 3–8 (`error`, `value`, `domain`, `output`, `transform`) PASS. `shorthand` is still an empty stub and `lib.rs` still has no public API — both compile fine.

- [ ] **Step 6: Commit**

```bash
git add rust/ndsel/src/transform.rs
git commit -m "feat(rust): Transform canonicalization (explicit identity output, idempotent)"
```

---

## Task 9: `shorthand.rs` — `point` and `box` desugaring

**Files:**
- Create: `rust/ndsel/src/shorthand.rs`
- Modify: `rust/ndsel/src/lib.rs` (add the public API: `Message`, `normalize`, `parse`, re-exports)
- Test: inline in `shorthand.rs`

- [ ] **Step 1: Write the failing test**

Create `rust/ndsel/src/shorthand.rs` with this test at the bottom:

```rust
#[cfg(test)]
mod point_box_tests {
    use super::*;

    fn norm(json: &str) -> serde_json::Value {
        let msg = crate::parse(json).unwrap();
        serde_json::to_value(crate::normalize(msg).unwrap()).unwrap()
    }

    #[test]
    fn point_desugars_to_constant_maps() {
        let v = norm(r#"{ "kind": "point", "coords": [4, 7] }"#);
        assert_eq!(v["input_rank"], 0);
        assert_eq!(v["input_inclusive_min"], serde_json::json!([]));
        assert_eq!(v["input_exclusive_max"], serde_json::json!([]));
        assert_eq!(
            v["output"],
            serde_json::json!([{ "offset": 4 }, { "offset": 7 }])
        );
    }

    #[test]
    fn box_desugars_to_identity() {
        let v = norm(r#"{ "kind": "box", "inclusive_min": [0, 0], "exclusive_max": [3, 4] }"#);
        assert_eq!(v["input_inclusive_min"], serde_json::json!([0, 0]));
        assert_eq!(v["input_exclusive_max"], serde_json::json!([3, 4]));
        assert_eq!(
            v["output"],
            serde_json::json!([
                { "offset": 0, "stride": 1, "input_dimension": 0 },
                { "offset": 0, "stride": 1, "input_dimension": 1 }
            ])
        );
    }

    #[test]
    fn box_shape_only_defaults_origin_zero() {
        let v = norm(r#"{ "kind": "box", "shape": [5] }"#);
        assert_eq!(v["input_inclusive_min"], serde_json::json!([0]));
        assert_eq!(v["input_exclusive_max"], serde_json::json!([5]));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ndsel --lib point_box 2>&1 | tail -20`
Expected: FAIL — `Point` / `BoxSel` not defined.

- [ ] **Step 3: Write the implementation**

Add ABOVE the test module in `rust/ndsel/src/shorthand.rs`:

```rust
use serde::Deserialize;

use crate::domain::{Domain, RawDomain};
use crate::error::NdsqError;
use crate::output::OutputMap;
use crate::transform::{from_parts, Transform};
use crate::value::{ImplicitValue, IndexValue};

/// `{ "kind": "point", "coords": [...] }`
#[derive(Debug, Clone, Deserialize)]
pub struct Point {
    pub coords: Vec<i64>,
}

impl Point {
    pub fn desugar(self) -> Result<Transform, NdsqError> {
        let domain = Domain {
            rank: 0,
            inclusive_min: vec![],
            exclusive_max: vec![],
            labels: vec![],
        };
        let output = self
            .coords
            .into_iter()
            .map(|c| OutputMap::Constant { offset: c })
            .collect();
        Ok(from_parts(domain, output))
    }
}

/// `{ "kind": "box", inclusive_min/exclusive_max/inclusive_max/shape, labels? }`
#[derive(Debug, Clone, Deserialize)]
pub struct BoxSel {
    #[serde(flatten)]
    domain: RawDomain,
}

impl BoxSel {
    pub fn desugar(self) -> Result<Transform, NdsqError> {
        let domain = self.domain.into_domain()?;
        let output = (0..domain.rank)
            .map(|k| OutputMap::SingleInputDimension { offset: 0, stride: 1, input_dimension: k })
            .collect();
        Ok(from_parts(domain, output))
    }
}

// Placeholders filled in Tasks 10–11 so `lib.rs` compiles. Replace these in those tasks.
#[derive(Debug, Clone, Deserialize)]
pub struct Slice {
    pub start: Vec<i64>,
    pub stop: Vec<i64>,
    #[serde(default)]
    pub step: Option<Vec<i64>>,
    #[serde(default)]
    pub labels: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Points {
    pub coords: Vec<Vec<i64>>,
}

impl Slice {
    pub fn desugar(self) -> Result<Transform, NdsqError> {
        // Implemented in Task 10.
        let _ = (&self.start, &self.stop, &self.step, &self.labels);
        unimplemented!("Slice::desugar — Task 10")
    }
}

impl Points {
    pub fn desugar(self) -> Result<Transform, NdsqError> {
        // Implemented in Task 11.
        let _ = &self.coords;
        unimplemented!("Points::desugar — Task 11")
    }
}

// Used by slice/points desugaring helpers.
#[allow(dead_code)]
fn fin(n: i64) -> ImplicitValue { ImplicitValue::explicit(IndexValue::Finite(n)) }
```

Now add the public API to `rust/ndsel/src/lib.rs` (append below the `mod` declarations — every referenced type now exists):

```rust
pub use domain::Domain;
pub use error::{NdsqError, Reason};
pub use output::OutputMap;
pub use transform::Transform;
pub use value::{ImplicitValue, IndexValue};

/// A spatial-query message, discriminated by its `kind`.
///
/// Deliberately NOT a `#[serde(tag = "kind")]` enum: internally-tagged enums
/// combine poorly with `#[serde(flatten)]` (used by `BoxSel`). `parse` peeks
/// `kind` and dispatches manually, which is robust and yields a clean
/// `unknown_kind` error.
#[derive(Debug, Clone)]
pub enum Message {
    Point(shorthand::Point),
    Box(shorthand::BoxSel),
    Slice(shorthand::Slice),
    Points(shorthand::Points),
    Transform(Transform),
}

/// Reduce any message to its canonical `Transform`.
pub fn normalize(message: Message) -> Result<Transform, NdsqError> {
    match message {
        Message::Point(p) => p.desugar(),
        Message::Box(b) => b.desugar(),
        Message::Slice(s) => s.desugar(),
        Message::Points(p) => p.desugar(),
        Message::Transform(t) => t.canonicalize(),
    }
}

/// Parse a JSON string into a `Message`, dispatching on the `kind` discriminator.
/// This is the final version; `unknown_kind` and missing-`kind` handling are
/// complete here (Task 12 only adds regression tests).
pub fn parse(json: &str) -> Result<Message, NdsqError> {
    use serde_json::{from_value, Value};
    let value: Value = serde_json::from_str(json).map_err(NdsqError::from_serde)?;
    let kind = value
        .get("kind")
        .and_then(|k| k.as_str())
        .ok_or_else(|| NdsqError::new(Reason::InvalidJson, "missing string `kind`"))?
        .to_owned();
    let msg = match kind.as_str() {
        "point" => Message::Point(from_value(value).map_err(NdsqError::from_serde)?),
        "box" => Message::Box(from_value(value).map_err(NdsqError::from_serde)?),
        "slice" => Message::Slice(from_value(value).map_err(NdsqError::from_serde)?),
        "points" => Message::Points(from_value(value).map_err(NdsqError::from_serde)?),
        "transform" => Message::Transform(from_value(value).map_err(NdsqError::from_serde)?),
        other => {
            return Err(NdsqError::new(Reason::UnknownKind, format!("unknown kind: {other}")));
        }
    };
    Ok(msg)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p ndsel --lib point_box 2>&1 | tail -20`
Expected: 3 tests PASS. (`Slice`/`Points` compile but `unimplemented!` — not yet exercised.)

- [ ] **Step 5: Commit**

```bash
git add rust/ndsel/src/shorthand.rs rust/ndsel/src/lib.rs
git commit -m "feat(rust): desugar point and box shorthands"
```

---

## Task 10: `shorthand.rs` — `slice` desugaring (positive step)

**Files:**
- Modify: `rust/ndsel/src/shorthand.rs`
- Test: inline in `shorthand.rs`

Implements spec §6.3 for `s > 0`: per dim `(a, b, s)` → `m = max(0, ceil((b-a)/s))`, origin `o = floor(a/s)`, `offset = a - s·o`, map `single_input_dimension(input_dimension=k, offset, stride=s)`, domain `[o, o+m)`. `s == 0` → `step_zero`; `s < 0` → `negative_step_unsupported`.

- [ ] **Step 1: Write the failing test**

Add a test module to `rust/ndsel/src/shorthand.rs`:

```rust
#[cfg(test)]
mod slice_tests {
    use super::*;

    fn norm(json: &str) -> serde_json::Value {
        let msg = crate::parse(json).unwrap();
        serde_json::to_value(crate::normalize(msg).unwrap()).unwrap()
    }
    fn err(json: &str) -> crate::Reason {
        let msg = crate::parse(json).unwrap();
        crate::normalize(msg).unwrap_err().reason
    }

    #[test]
    fn unit_step_preserves_source_frame() {
        // x[5:10] -> domain [5,10), identity map (offset 5? no: coordinate-preserving)
        let v = norm(r#"{ "kind": "slice", "start": [5], "stop": [10] }"#);
        assert_eq!(v["input_inclusive_min"], serde_json::json!([5]));
        assert_eq!(v["input_exclusive_max"], serde_json::json!([10]));
        // offset = 5 - 1*5 = 0, stride 1, dim 0 -> identity in the source frame
        assert_eq!(
            v["output"],
            serde_json::json!([{ "offset": 0, "stride": 1, "input_dimension": 0 }])
        );
    }

    #[test]
    fn divisible_strided_slice() {
        // x[4:10:2] selects {4,6,8}; o=2, offset=0, domain [2,5)
        let v = norm(r#"{ "kind": "slice", "start": [4], "stop": [10], "step": [2] }"#);
        assert_eq!(v["input_inclusive_min"], serde_json::json!([2]));
        assert_eq!(v["input_exclusive_max"], serde_json::json!([5]));
        assert_eq!(
            v["output"],
            serde_json::json!([{ "offset": 0, "stride": 2, "input_dimension": 0 }])
        );
    }

    #[test]
    fn nondivisible_strided_slice_uses_phase_offset() {
        // x[5:10:2] selects {5,7,9}; o=floor(5/2)=2, offset=5-2*2=1, domain [2,5)
        let v = norm(r#"{ "kind": "slice", "start": [5], "stop": [10], "step": [2] }"#);
        assert_eq!(v["input_inclusive_min"], serde_json::json!([2]));
        assert_eq!(v["input_exclusive_max"], serde_json::json!([5]));
        assert_eq!(
            v["output"],
            serde_json::json!([{ "offset": 1, "stride": 2, "input_dimension": 0 }])
        );
    }

    #[test]
    fn empty_slice_has_zero_length_dim() {
        // x[10:10] -> m=0, o=10, domain [10,10)
        let v = norm(r#"{ "kind": "slice", "start": [10], "stop": [10] }"#);
        assert_eq!(v["input_inclusive_min"], serde_json::json!([10]));
        assert_eq!(v["input_exclusive_max"], serde_json::json!([10]));
    }

    #[test]
    fn step_zero_is_error() {
        assert_eq!(err(r#"{ "kind": "slice", "start": [0], "stop": [4], "step": [0] }"#), crate::Reason::StepZero);
    }

    #[test]
    fn negative_step_is_unsupported() {
        assert_eq!(
            err(r#"{ "kind": "slice", "start": [9], "stop": [-1], "step": [-2] }"#),
            crate::Reason::NegativeStepUnsupported
        );
    }

    #[test]
    fn slice_length_mismatch_is_rank_mismatch() {
        assert_eq!(
            err(r#"{ "kind": "slice", "start": [0, 0], "stop": [4] }"#),
            crate::Reason::RankMismatch
        );
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ndsel --lib slice_tests 2>&1 | tail -20`
Expected: FAIL — `unimplemented!` panics / assertions fail.

- [ ] **Step 3: Write the implementation**

Replace `Slice::desugar` in `rust/ndsel/src/shorthand.rs` with:

```rust
impl Slice {
    pub fn desugar(self) -> Result<Transform, NdsqError> {
        let rank = self.start.len();
        if self.stop.len() != rank {
            return Err(NdsqError::new(
                crate::error::Reason::RankMismatch,
                "start and stop must have equal length",
            ));
        }
        let step: Vec<i64> = match self.step {
            Some(s) => {
                if s.len() != rank {
                    return Err(NdsqError::new(
                        crate::error::Reason::RankMismatch,
                        "step length must match start/stop",
                    ));
                }
                s
            }
            None => vec![1; rank],
        };
        if let Some(labels) = &self.labels {
            if labels.len() != rank {
                return Err(NdsqError::new(
                    crate::error::Reason::RankMismatch,
                    "labels length must match start/stop",
                ));
            }
        }

        let mut inclusive_min = Vec::with_capacity(rank);
        let mut exclusive_max = Vec::with_capacity(rank);
        let mut output = Vec::with_capacity(rank);

        for k in 0..rank {
            let (a, b, s) = (self.start[k], self.stop[k], step[k]);
            if s == 0 {
                return Err(NdsqError::new(crate::error::Reason::StepZero, "step must be non-zero"));
            }
            if s < 0 {
                return Err(NdsqError::new(
                    crate::error::Reason::NegativeStepUnsupported,
                    "negative step is not yet specified",
                ));
            }
            // s > 0
            let m = if b <= a { 0 } else { ceil_div(b - a, s) };
            let o = a.div_euclid(s); // floor(a/s) for s > 0
            let offset = a - s * o; // == a.rem_euclid(s), the lattice phase in [0, s)
            inclusive_min.push(fin(o));
            exclusive_max.push(fin(o + m));
            output.push(OutputMap::SingleInputDimension {
                offset,
                stride: s,
                input_dimension: k,
            });
        }

        let labels = self.labels.unwrap_or_else(|| vec![String::new(); rank]);
        let domain = Domain { rank, inclusive_min, exclusive_max, labels };
        Ok(from_parts(domain, output))
    }
}

/// Ceiling of p/q for p >= 0, q > 0.
fn ceil_div(p: i64, q: i64) -> i64 {
    (p + q - 1) / q
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p ndsel --lib slice_tests 2>&1 | tail -20`
Expected: 7 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/ndsel/src/shorthand.rs
git commit -m "feat(rust): desugar slice (positive step, coordinate-preserving)"
```

---

## Task 11: `shorthand.rs` — `points` desugaring (row-major → columnar)

**Files:**
- Modify: `rust/ndsel/src/shorthand.rs`
- Test: inline in `shorthand.rs`

Implements spec §6.4: `m` points of rank `n` → `input_rank: 1`, domain `[0, m)`, `n` index-array maps, each `index_array` the column `k` across all points.

- [ ] **Step 1: Write the failing test**

Add a test module to `rust/ndsel/src/shorthand.rs`:

```rust
#[cfg(test)]
mod points_tests {
    use super::*;

    fn norm(json: &str) -> serde_json::Value {
        let msg = crate::parse(json).unwrap();
        serde_json::to_value(crate::normalize(msg).unwrap()).unwrap()
    }

    #[test]
    fn points_transpose_to_columnar_index_arrays() {
        // three 2-D points: (1,10), (2,20), (3,30)
        let v = norm(r#"{ "kind": "points", "coords": [[1, 10], [2, 20], [3, 30]] }"#);
        assert_eq!(v["input_rank"], 1);
        assert_eq!(v["input_inclusive_min"], serde_json::json!([0]));
        assert_eq!(v["input_exclusive_max"], serde_json::json!([3]));
        assert_eq!(
            v["output"],
            serde_json::json!([
                { "offset": 0, "stride": 1, "index_array": [1, 2, 3],   "index_array_bounds": ["-inf", "+inf"] },
                { "offset": 0, "stride": 1, "index_array": [10, 20, 30], "index_array_bounds": ["-inf", "+inf"] }
            ])
        );
    }

    #[test]
    fn empty_points_is_zero_length_with_unknown_rank_zero() {
        // No points -> m=0, output rank 0 (no columns to emit)
        let v = norm(r#"{ "kind": "points", "coords": [] }"#);
        assert_eq!(v["input_rank"], 1);
        assert_eq!(v["input_exclusive_max"], serde_json::json!([0]));
        assert_eq!(v["output"], serde_json::json!([]));
    }

    #[test]
    fn ragged_points_is_rank_mismatch() {
        let msg = crate::parse(r#"{ "kind": "points", "coords": [[1, 2], [3]] }"#).unwrap();
        assert_eq!(crate::normalize(msg).unwrap_err().reason, crate::Reason::RankMismatch);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ndsel --lib points_tests 2>&1 | tail -20`
Expected: FAIL — `unimplemented!` panics.

- [ ] **Step 3: Write the implementation**

Replace `Points::desugar` in `rust/ndsel/src/shorthand.rs` with:

```rust
impl Points {
    pub fn desugar(self) -> Result<Transform, NdsqError> {
        let m = self.coords.len();
        // Output rank = point dimensionality (0 if there are no points).
        let n = self.coords.first().map(|p| p.len()).unwrap_or(0);
        for p in &self.coords {
            if p.len() != n {
                return Err(NdsqError::new(
                    crate::error::Reason::RankMismatch,
                    "all points must have equal dimensionality",
                ));
            }
        }

        // input domain: single dimension [0, m)
        let domain = Domain {
            rank: 1,
            inclusive_min: vec![fin(0)],
            exclusive_max: vec![fin(m as i64)],
            labels: vec![String::new()],
        };

        // one index_array map per output dimension k: column k across all points
        let mut output = Vec::with_capacity(n);
        for k in 0..n {
            let column: Vec<serde_json::Value> = self
                .coords
                .iter()
                .map(|p| serde_json::Value::from(p[k]))
                .collect();
            output.push(OutputMap::IndexArray {
                offset: 0,
                stride: 1,
                index_array: serde_json::Value::Array(column),
                bounds: (IndexValue::NegInf, IndexValue::PosInf),
            });
        }

        Ok(from_parts(domain, output))
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p ndsel --lib points_tests 2>&1 | tail -20`
Expected: 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/ndsel/src/shorthand.rs
git commit -m "feat(rust): desugar points (row-major to columnar index arrays)"
```

---

## Task 12: Regression tests for `kind` dispatch

**Files:**
- Modify: `rust/ndsel/src/lib.rs` (add tests only)
- Test: inline in `lib.rs`

`parse` (Task 2) already dispatches on `kind` and returns `unknown_kind` / `invalid_json`. These tests lock that behavior in.

- [ ] **Step 1: Write the tests**

Add to the bottom of `rust/ndsel/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_kind_maps_to_reason() {
        let err = parse(r#"{ "kind": "bogus" }"#).unwrap_err();
        assert_eq!(err.reason, Reason::UnknownKind);
    }

    #[test]
    fn missing_kind_is_invalid_json() {
        let err = parse(r#"{ "coords": [1] }"#).unwrap_err();
        assert_eq!(err.reason, Reason::InvalidJson);
    }

    #[test]
    fn malformed_json_is_invalid_json() {
        let err = parse(r#"{ not json"#).unwrap_err();
        assert_eq!(err.reason, Reason::InvalidJson);
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p ndsel --lib tests 2>&1 | tail -20`
Expected: all three PASS immediately (behavior implemented by `parse` in Task 9). If any fail, reconcile `parse` with Task 9's version rather than weakening the test.

- [ ] **Step 3: Run the full lib suite**

Run: `cargo test -p ndsel --lib 2>&1 | tail -20`
Expected: ALL lib tests across Tasks 3–12 PASS.

- [ ] **Step 4: Commit**

```bash
git add rust/ndsel/src/lib.rs
git commit -m "test(rust): regression tests for kind dispatch"
```

---

## Task 13: JSON Schema

**Files:**
- Create: `schema/ndsel.schema.json`

Validates **structure only** (spec §7). The discriminated union keys on `kind`; per-variant required fields; the `[n]`-bracket implicit form is `oneOf: [<scalar>, <1-element array of scalar>]`; the three upper-bound spellings are mutually exclusive in `box`/`transform`.

- [ ] **Step 1: Write the schema**

Create `schema/ndsel.schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://ndsel.dev/schema/ndsel.schema.json",
  "title": "ndsel message",
  "$defs": {
    "indexValue": {
      "oneOf": [
        { "type": "integer" },
        { "type": "string", "enum": ["-inf", "+inf"] }
      ]
    },
    "implicitValue": {
      "oneOf": [
        { "$ref": "#/$defs/indexValue" },
        { "type": "array", "items": { "$ref": "#/$defs/indexValue" }, "minItems": 1, "maxItems": 1 }
      ]
    },
    "boundArray": { "type": "array", "items": { "$ref": "#/$defs/implicitValue" } },
    "outputMap": {
      "type": "object",
      "properties": {
        "offset": { "type": "integer" },
        "stride": { "type": "integer" },
        "input_dimension": { "type": "integer", "minimum": 0 },
        "index_array": {},
        "index_array_bounds": {
          "type": "array",
          "items": { "$ref": "#/$defs/indexValue" },
          "minItems": 2, "maxItems": 2
        }
      },
      "not": { "required": ["input_dimension", "index_array"] },
      "additionalProperties": false
    },
    "upperBoundExclusivity": {
      "allOf": [
        { "not": { "required": ["exclusive_max", "inclusive_max"] } },
        { "not": { "required": ["exclusive_max", "shape"] } },
        { "not": { "required": ["inclusive_max", "shape"] } }
      ]
    },
    "inputUpperBoundExclusivity": {
      "allOf": [
        { "not": { "required": ["input_exclusive_max", "input_inclusive_max"] } },
        { "not": { "required": ["input_exclusive_max", "input_shape"] } },
        { "not": { "required": ["input_inclusive_max", "input_shape"] } }
      ]
    }
  },
  "oneOf": [
    {
      "type": "object",
      "properties": { "kind": { "const": "point" }, "coords": { "type": "array", "items": { "type": "integer" } } },
      "required": ["kind", "coords"], "additionalProperties": false
    },
    {
      "type": "object",
      "allOf": [{ "$ref": "#/$defs/upperBoundExclusivity" }],
      "properties": {
        "kind": { "const": "box" },
        "inclusive_min": { "$ref": "#/$defs/boundArray" },
        "exclusive_max": { "$ref": "#/$defs/boundArray" },
        "inclusive_max": { "$ref": "#/$defs/boundArray" },
        "shape": { "$ref": "#/$defs/boundArray" },
        "labels": { "type": "array", "items": { "type": "string" } }
      },
      "required": ["kind"], "additionalProperties": false
    },
    {
      "type": "object",
      "properties": {
        "kind": { "const": "slice" },
        "start": { "type": "array", "items": { "type": "integer" } },
        "stop": { "type": "array", "items": { "type": "integer" } },
        "step": { "type": "array", "items": { "type": "integer" } },
        "labels": { "type": "array", "items": { "type": "string" } }
      },
      "required": ["kind", "start", "stop"], "additionalProperties": false
    },
    {
      "type": "object",
      "properties": {
        "kind": { "const": "points" },
        "coords": { "type": "array", "items": { "type": "array", "items": { "type": "integer" } } }
      },
      "required": ["kind", "coords"], "additionalProperties": false
    },
    {
      "type": "object",
      "allOf": [{ "$ref": "#/$defs/inputUpperBoundExclusivity" }],
      "properties": {
        "kind": { "const": "transform" },
        "input_rank": { "type": "integer", "minimum": 0, "maximum": 32 },
        "input_inclusive_min": { "$ref": "#/$defs/boundArray" },
        "input_exclusive_max": { "$ref": "#/$defs/boundArray" },
        "input_inclusive_max": { "$ref": "#/$defs/boundArray" },
        "input_shape": { "$ref": "#/$defs/boundArray" },
        "input_labels": { "type": "array", "items": { "type": "string" } },
        "output": { "type": "array", "items": { "$ref": "#/$defs/outputMap" } }
      },
      "required": ["kind"], "additionalProperties": false
    }
  ]
}
```

Note: `box` uses the un-prefixed `upperBoundExclusivity`; `transform` uses `inputUpperBoundExclusivity` (both defined in `$defs`). The `index_array` schema is intentionally permissive (`{}`) — deep nested-array validation is out of scope (spec §12).

- [ ] **Step 2: Commit**

```bash
git add schema/ndsel.schema.json
git commit -m "feat: JSON Schema for ndsel messages (syntax validation)"
```

---

## Task 14: Conformance corpus + Rust runner

**Files:**
- Create: `conformance/README.md`
- Create: `conformance/point.json`, `box.json`, `slice.json`, `points.json`, `transform.json`, `errors.json`
- Create: `rust/ndsel/tests/conformance.rs`

- [ ] **Step 1: Write the corpus README (fixture format)**

Create `conformance/README.md`:

```markdown
# ndsel conformance corpus

Language-agnostic fixtures. Each file is a JSON array of cases.

A **success** case:
    { "name": "...", "input": <message>, "normalized": <canonical transform without `kind`> }

An **error** case:
    { "name": "...", "input": <message>, "error": "<reason_code>" }

An implementation is conformant iff, for every success case,
`normalize(input)` equals `normalized` by structural JSON equality, and for
every error case, `normalize(input)` is rejected with the given reason code.

The `normalized` value is a canonical `transform` body (the `kind` field is
omitted; implementations compare the transform structure).
```

- [ ] **Step 2: Write the failing runner test**

Create `rust/ndsel/tests/conformance.rs`:

```rust
//! Runs every fixture in ../../conformance against `ndsel::normalize`,
//! and validates each `input` against the JSON Schema.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

fn repo_root() -> PathBuf {
    // tests run with CWD = rust/ndsel; corpus + schema are two levels up.
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn load_schema() -> jsonschema::Validator {
    let path = repo_root().join("schema/ndsel.schema.json");
    let schema: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    jsonschema::validator_for(&schema).expect("schema compiles")
}

fn corpus_files() -> Vec<PathBuf> {
    let dir = repo_root().join("conformance");
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "json").unwrap_or(false))
        .collect();
    files.sort();
    files
}

/// Strip the `kind` field for comparison: corpus `normalized` omits it.
fn without_kind(mut v: Value) -> Value {
    if let Value::Object(map) = &mut v {
        map.remove("kind");
    }
    v
}

#[test]
fn corpus_round_trips() {
    let schema = load_schema();
    let mut count = 0usize;

    for file in corpus_files() {
        let cases: Vec<Value> =
            serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        for case in cases {
            let name = case["name"].as_str().unwrap_or("<unnamed>");
            let input = &case["input"];
            let msg = ndsel::parse(&input.to_string());

            if let Some(expected_reason) = case.get("error").and_then(|e| e.as_str()) {
                // Error inputs may be intentionally schema-invalid, so they are
                // NOT schema-checked here.
                let err = match msg {
                    Ok(m) => ndsel::normalize(m).expect_err(&format!("{name}: expected error")),
                    Err(e) => e,
                };
                assert_eq!(err.reason.code(), expected_reason, "{}: wrong reason", name);
            } else {
                // Every success input MUST satisfy the schema (syntax).
                assert!(schema.is_valid(input), "{}: input fails schema", name);
                let normalized = ndsel::normalize(msg.expect(name)).expect(name);
                let got = without_kind(serde_json::to_value(&normalized).unwrap());
                let want = case["normalized"].clone();
                assert_eq!(got, want, "{}: normalized mismatch", name);
            }
            count += 1;
        }
    }
    assert!(count >= 15, "expected a populated corpus, ran {count}");
}
```

- [ ] **Step 3: Run runner to verify it fails**

Run: `cargo test -p ndsel --test conformance 2>&1 | tail -30`
Expected: FAIL — corpus files don't exist yet (read_dir empty or `count < 15`).

- [ ] **Step 4: Author the corpus fixtures**

For each file, derive the `normalized` body **by hand from spec §5–6** (do not copy runtime output — the corpus is the spec's ground truth; the runner proves the implementation matches it). Each case's `normalized` is the canonical transform **without** `kind`: object with `input_rank`, `input_inclusive_min`, `input_exclusive_max`, `input_labels`, `output`.

Create `conformance/point.json`:

```json
[
  {
    "name": "point/2d",
    "input": { "kind": "point", "coords": [4, 7] },
    "normalized": {
      "input_rank": 0,
      "input_inclusive_min": [],
      "input_exclusive_max": [],
      "input_labels": [],
      "output": [ { "offset": 4 }, { "offset": 7 } ]
    }
  },
  {
    "name": "point/scalar-0d",
    "input": { "kind": "point", "coords": [] },
    "normalized": {
      "input_rank": 0, "input_inclusive_min": [], "input_exclusive_max": [],
      "input_labels": [], "output": []
    }
  }
]
```

Create `conformance/box.json`:

```json
[
  {
    "name": "box/2d-min-max",
    "input": { "kind": "box", "inclusive_min": [0, 0], "exclusive_max": [3, 4] },
    "normalized": {
      "input_rank": 2,
      "input_inclusive_min": [0, 0],
      "input_exclusive_max": [3, 4],
      "input_labels": ["", ""],
      "output": [
        { "offset": 0, "stride": 1, "input_dimension": 0 },
        { "offset": 0, "stride": 1, "input_dimension": 1 }
      ]
    }
  },
  {
    "name": "box/shape-only-origin-zero",
    "input": { "kind": "box", "shape": [5] },
    "normalized": {
      "input_rank": 1,
      "input_inclusive_min": [0],
      "input_exclusive_max": [5],
      "input_labels": [""],
      "output": [ { "offset": 0, "stride": 1, "input_dimension": 0 } ]
    }
  },
  {
    "name": "box/inclusive-max",
    "input": { "kind": "box", "inclusive_min": [2], "inclusive_max": [9] },
    "normalized": {
      "input_rank": 1, "input_inclusive_min": [2], "input_exclusive_max": [10],
      "input_labels": [""],
      "output": [ { "offset": 0, "stride": 1, "input_dimension": 0 } ]
    }
  }
]
```

Create `conformance/slice.json`:

```json
[
  {
    "name": "slice/unit-step-preserves-frame",
    "input": { "kind": "slice", "start": [5], "stop": [10] },
    "normalized": {
      "input_rank": 1, "input_inclusive_min": [5], "input_exclusive_max": [10],
      "input_labels": [""],
      "output": [ { "offset": 0, "stride": 1, "input_dimension": 0 } ]
    }
  },
  {
    "name": "slice/divisible-stride",
    "input": { "kind": "slice", "start": [4], "stop": [10], "step": [2] },
    "normalized": {
      "input_rank": 1, "input_inclusive_min": [2], "input_exclusive_max": [5],
      "input_labels": [""],
      "output": [ { "offset": 0, "stride": 2, "input_dimension": 0 } ]
    }
  },
  {
    "name": "slice/nondivisible-stride-phase-offset",
    "input": { "kind": "slice", "start": [5], "stop": [10], "step": [2] },
    "normalized": {
      "input_rank": 1, "input_inclusive_min": [2], "input_exclusive_max": [5],
      "input_labels": [""],
      "output": [ { "offset": 1, "stride": 2, "input_dimension": 0 } ]
    }
  }
]
```

Create `conformance/points.json`:

```json
[
  {
    "name": "points/three-2d",
    "input": { "kind": "points", "coords": [[1, 10], [2, 20], [3, 30]] },
    "normalized": {
      "input_rank": 1, "input_inclusive_min": [0], "input_exclusive_max": [3],
      "input_labels": [""],
      "output": [
        { "offset": 0, "stride": 1, "index_array": [1, 2, 3], "index_array_bounds": ["-inf", "+inf"] },
        { "offset": 0, "stride": 1, "index_array": [10, 20, 30], "index_array_bounds": ["-inf", "+inf"] }
      ]
    }
  }
]
```

Create `conformance/transform.json`:

```json
[
  {
    "name": "transform/omitted-output-identity",
    "input": { "kind": "transform", "input_inclusive_min": [0, 0], "input_exclusive_max": [3, 4] },
    "normalized": {
      "input_rank": 2, "input_inclusive_min": [0, 0], "input_exclusive_max": [3, 4],
      "input_labels": ["", ""],
      "output": [
        { "offset": 0, "stride": 1, "input_dimension": 0 },
        { "offset": 0, "stride": 1, "input_dimension": 1 }
      ]
    }
  },
  {
    "name": "transform/implicit-bounds-and-labels",
    "input": {
      "kind": "transform",
      "input_inclusive_min": [["-inf"], 7],
      "input_exclusive_max": [["+inf"], 11],
      "input_labels": ["x", "y"]
    },
    "normalized": {
      "input_rank": 2,
      "input_inclusive_min": [["-inf"], 7],
      "input_exclusive_max": [["+inf"], 11],
      "input_labels": ["x", "y"],
      "output": [
        { "offset": 0, "stride": 1, "input_dimension": 0 },
        { "offset": 0, "stride": 1, "input_dimension": 1 }
      ]
    }
  }
]
```

Create `conformance/errors.json`:

```json
[
  { "name": "error/step-zero", "input": { "kind": "slice", "start": [0], "stop": [4], "step": [0] }, "error": "step_zero" },
  { "name": "error/negative-step", "input": { "kind": "slice", "start": [9], "stop": [0], "step": [-2] }, "error": "negative_step_unsupported" },
  { "name": "error/multiple-upper-bounds", "input": { "kind": "box", "shape": [3], "exclusive_max": [3] }, "error": "multiple_upper_bounds" },
  { "name": "error/rank-mismatch", "input": { "kind": "slice", "start": [0, 0], "stop": [4] }, "error": "rank_mismatch" },
  { "name": "error/unknown-kind", "input": { "kind": "bogus" }, "error": "unknown_kind" }
]
```

Note: the `error/multiple-upper-bounds` and `error/unknown-kind` inputs are intentionally schema-INVALID. The runner (Step 2) only schema-checks success cases, so this is expected and fine.

- [ ] **Step 5: Run the runner to verify it passes**

Run: `cargo test -p ndsel --test conformance 2>&1 | tail -30`
Expected: PASS — all success fixtures normalize to their `normalized` value and all error fixtures reject with the stated reason code.

- [ ] **Step 6: Commit**

```bash
git add conformance/ rust/ndsel/tests/conformance.rs
git commit -m "test: conformance corpus and Rust runner (schema-validate + normalize)"
```

---

## Task 15: README and final verification

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Write the top-level README**

Replace `README.md` with an overview: what ndsel is (one paragraph), the statement that **ndsel adapts the index model of Google tensorstore** (link the design doc), the five message kinds with a one-line example each, and pointers to `spec/ndsel.md`, `schema/ndsel.schema.json`, `conformance/`, and the three implementation directories (noting Rust is implemented; Python/TypeScript are Plans 2–3).

- [ ] **Step 2: Full verification**

Run: `cargo test -p ndsel 2>&1 | tail -20`
Expected: all lib tests AND the conformance runner PASS.

Run: `cargo build -p ndsel 2>&1 | tail -5`
Expected: clean build, no warnings about unused public items beyond the deferred `Reason::UnknownKind` wiring (now used).

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: top-level README pointing at spec, schema, corpus, and implementations"
```

---

## Self-review checklist (run after implementation)

- **Spec coverage:** every spec §5–6 desugaring has a Task (point=9, box=9, slice=10, points=11, transform=8) and a corpus fixture (Task 14). Errors (spec error list): reason codes=Task 3, `step_zero`/`negative_step_unsupported`/`rank_mismatch`=Task 10, `multiple_upper_bounds`/`rank_mismatch`=Task 6, `unknown_kind`/`invalid_json`=Task 9 `parse` (tested Task 12) + `errors.json`. Schema (§7) → Task 13. Corpus contract (§8) → Task 14.
- **Deferred items match spec §12:** negative step (Task 10 rejects with `negative_step_unsupported`), deep index_array validation (out of scope, noted in `output.rs`).
- **Type consistency:** `normalize`, `parse`, `Transform::canonicalize`, `*::desugar`, `RawDomain::into_domain`, `RawOutputMap::canonicalize`, `from_parts`, `Reason::code` are used with identical signatures across tasks.
- **No placeholders:** the only `unimplemented!`s are in Task 9 stubs, explicitly replaced in Tasks 10–11.
