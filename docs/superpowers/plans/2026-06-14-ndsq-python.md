# ndsq Python Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An idiomatic Python peer implementation of ndsq that passes the same `/conformance/` corpus as the Rust reference.

**Architecture:** Two layers. **TypedDicts** (`messages.py`) model the JSON message shapes — a `kind`-discriminated union, zero runtime overhead. **dataclasses** model both the ergonomic *input builders* (`Point`/`Box`/`Slice`/`Points`, each `.to_message()`) and the canonical *result* (`Transform`, `Domain`, the three output-map kinds, with `.to_dict()`). `parse(str) -> Message` validates and discriminates; `normalize(Message | builder) -> Transform` desugars to the canonical core. Runtime depends on stdlib only (`json`, `dataclasses`, `typing`); tests use `pytest` + `jsonschema`.

**Tech Stack:** Python 3.13, `uv`, `pytest`, `jsonschema` (test-only), hatchling build backend. Reference: `spec/ndsq.md`, the Rust crate `rust/ndsq/`, and the frozen corpus `conformance/`.

---

## Scope of this plan

- **In:** the full Python package mirroring the Rust reference — values/bounds, domain canonicalization, three output-map kinds, transform canonicalization, all four shorthand desugarers, TypedDict message types, dataclass builders, `parse`/`normalize`, error codes, and the conformance runner. Fix the broken scaffolding (`src/ndsq/__init__.py` is a **directory**; stray `main.py`).
- **Mirrors the frozen contract exactly.** The corpus and spec are NOT to be changed. If a corpus case fails, it is a Python bug — fix the Python, never the fixture.
- **Deferred (same as the contract):** negative `step` → `negative_step_unsupported`; deep `index_array` validation (carried as raw JSON). `box` accepts implicit/`±inf` bounds (it is an `IndexDomain`); `slice` is concrete-integer only.

## File structure

```
python/ndsq/pyproject.toml              project + dev deps (pytest, jsonschema) + hatchling
python/ndsq/src/ndsq/__init__.py        public API: parse, normalize, re-exports
python/ndsq/src/ndsq/errors.py          Reason (str Enum) + NdsqError
python/ndsq/src/ndsq/values.py          IndexValue alias, ImplicitValue dataclass, bound parse/emit
python/ndsq/src/ndsq/domain.py          Domain dataclass + canonicalize_domain
python/ndsq/src/ndsq/output.py          ConstantMap/SingleInputDimension/IndexArrayMap + canonicalize + to_json
python/ndsq/src/ndsq/transform.py       Transform dataclass + canonicalize_transform + identity + to_dict
python/ndsq/src/ndsq/messages.py        TypedDicts: Point/Box/Slice/Points/Transform message + Message union
python/ndsq/src/ndsq/builders.py        Point/Box/Slice/Points builder dataclasses (.to_message)
python/ndsq/src/ndsq/shorthand.py       desugar_point/box/slice/points
python/ndsq/tests/test_*.py             per-module unit tests (TDD)
python/ndsq/tests/test_conformance.py   corpus runner (schema-validate + normalize + compare)
```

Each module has one responsibility, mirroring the Rust crate's modules so the two implementations read as peers.

**Environment note for all tasks:** run every command from `/Users/d-v-b/dev/ndsq/python/ndsq`. Tests run via `uv run pytest`. The corpus and schema live at the repo root (`/Users/d-v-b/dev/ndsq/conformance`, `/Users/d-v-b/dev/ndsq/schema`), reachable from a test file as `Path(__file__).parents[3]`.

---

## Task 1: Fix scaffolding, configure pyproject

**Files:**
- Delete: `python/ndsq/main.py`, `python/ndsq/src/ndsq/__init__.py` (it is a stray **directory**)
- Modify: `python/ndsq/pyproject.toml`
- Create: `python/ndsq/src/ndsq/__init__.py` (empty for now), `python/ndsq/tests/__init__.py`

- [ ] **Step 1: Remove the broken scaffolding**

```bash
cd /Users/d-v-b/dev/ndsq/python/ndsq
rm -f main.py
rm -rf src/ndsq/__init__.py   # this is a directory, not a file
```

- [ ] **Step 2: Write pyproject.toml**

Replace `python/ndsq/pyproject.toml` with:

```toml
[project]
name = "ndsq"
version = "0.1.0"
description = "JSON-serializable n-dimensional spatial queries (Python implementation)"
readme = "README.md"
requires-python = ">=3.13"
dependencies = []

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[tool.hatch.build.targets.wheel]
packages = ["src/ndsq"]

[dependency-groups]
dev = ["pytest>=8", "jsonschema>=4.21"]

[tool.pytest.ini_options]
testpaths = ["tests"]
```

- [ ] **Step 3: Create the package and test-package init files**

Create `python/ndsq/src/ndsq/__init__.py` containing a single line:

```python
"""ndsq — JSON-serializable n-dimensional spatial queries."""
```

Create an empty `python/ndsq/tests/__init__.py` (zero bytes).

- [ ] **Step 4: Verify the toolchain resolves**

Run: `uv run python -c "import ndsq; print('ok')"`
Expected: prints `ok` (uv creates the venv, installs the package + dev group, import succeeds).

Run: `uv run pytest -q`
Expected: `no tests ran` (no tests yet) — exit code 5 is fine here; the point is pytest runs.

- [ ] **Step 5: Commit**

`git add -A` stages everything including the `main.py` deletion (it was tracked) and the `__init__.py` directory→file swap (the directory was an untracked empty dir):

```bash
git add -A python/ndsq
git commit -m "chore(py): fix scaffolding, configure pyproject for ndsq package"
```

---

## Task 2: `errors.py` — Reason codes and NdsqError

**Files:**
- Create: `python/ndsq/src/ndsq/errors.py`
- Test: `python/ndsq/tests/test_errors.py`

- [ ] **Step 1: Write the failing test**

Create `python/ndsq/tests/test_errors.py`:

```python
from ndsq.errors import NdsqError, Reason


def test_reason_codes_are_stable():
    assert Reason.STEP_ZERO.value == "step_zero"
    assert Reason.NEGATIVE_STEP_UNSUPPORTED.value == "negative_step_unsupported"
    assert Reason.MULTIPLE_UPPER_BOUNDS.value == "multiple_upper_bounds"
    assert Reason.RANK_MISMATCH.value == "rank_mismatch"
    assert Reason.UNKNOWN_KIND.value == "unknown_kind"
    assert Reason.INVALID_JSON.value == "invalid_json"


def test_error_carries_reason_and_detail():
    err = NdsqError(Reason.STEP_ZERO, "step must be non-zero")
    assert err.reason is Reason.STEP_ZERO
    assert "step must be non-zero" in str(err)
```

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run pytest tests/test_errors.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'ndsq.errors'`.

- [ ] **Step 3: Write the implementation**

Create `python/ndsq/src/ndsq/errors.py`:

```python
"""Stable error codes and the exception type for ndsq."""

from __future__ import annotations

from enum import Enum


class Reason(str, Enum):
    """Machine-readable reason codes for rejected messages (the wire codes)."""

    STEP_ZERO = "step_zero"
    NEGATIVE_STEP_UNSUPPORTED = "negative_step_unsupported"
    MULTIPLE_UPPER_BOUNDS = "multiple_upper_bounds"
    RANK_MISMATCH = "rank_mismatch"
    UNKNOWN_KIND = "unknown_kind"
    INVALID_JSON = "invalid_json"


class NdsqError(Exception):
    """Raised when a message cannot be parsed or normalized."""

    def __init__(self, reason: Reason, detail: str) -> None:
        super().__init__(f"{reason.value}: {detail}")
        self.reason = reason
        self.detail = detail
```

- [ ] **Step 4: Run test to verify it passes**

Run: `uv run pytest tests/test_errors.py -q`
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add python/ndsq/src/ndsq/errors.py python/ndsq/tests/test_errors.py
git commit -m "feat(py): Reason codes and NdsqError"
```

---

## Task 3: `values.py` — IndexValue and ImplicitValue

**Files:**
- Create: `python/ndsq/src/ndsq/values.py`
- Test: `python/ndsq/tests/test_values.py`

`IndexValue` is `int | "-inf" | "+inf"`. `ImplicitValue` wraps one plus an `implicit` flag, with JSON parse/emit following the `[n]`-bracket convention. **Python gotcha:** `bool` is a subclass of `int`, so `True`/`False` must be rejected as index values.

- [ ] **Step 1: Write the failing test**

Create `python/ndsq/tests/test_values.py`:

```python
import pytest

from ndsq.errors import NdsqError, Reason
from ndsq.values import ImplicitValue, parse_index_value


def test_parse_index_value_int_and_sentinels():
    assert parse_index_value(7) == 7
    assert parse_index_value("-inf") == "-inf"
    assert parse_index_value("+inf") == "+inf"


def test_parse_index_value_rejects_bool_and_garbage():
    with pytest.raises(NdsqError) as e1:
        parse_index_value(True)
    assert e1.value.reason is Reason.INVALID_JSON
    with pytest.raises(NdsqError):
        parse_index_value("nope")


def test_bare_value_is_explicit():
    v = ImplicitValue.from_json(7)
    assert v.value == 7 and v.implicit is False
    assert v.to_json() == 7


def test_bracketed_value_is_implicit():
    v = ImplicitValue.from_json([7])
    assert v.value == 7 and v.implicit is True
    assert v.to_json() == [7]


def test_bracketed_sentinel_is_implicit():
    v = ImplicitValue.from_json(["-inf"])
    assert v.value == "-inf" and v.implicit is True
    assert v.to_json() == ["-inf"]


def test_multi_element_bracket_is_invalid():
    with pytest.raises(NdsqError) as e:
        ImplicitValue.from_json([1, 2])
    assert e.value.reason is Reason.INVALID_JSON


def test_explicit_constructor():
    assert ImplicitValue.explicit(0) == ImplicitValue(value=0, implicit=False)
```

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run pytest tests/test_values.py -q`
Expected: FAIL — `No module named 'ndsq.values'`.

- [ ] **Step 3: Write the implementation**

Create `python/ndsq/src/ndsq/values.py`:

```python
"""Index values (finite or +/-inf) and bounds with the [n]-bracket implicit convention."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal, Union

from .errors import NdsqError, Reason

# A single index coordinate: a finite int, or the infinity sentinels.
IndexValue = Union[int, Literal["-inf", "+inf"]]


def parse_index_value(raw: object) -> IndexValue:
    """Validate a JSON scalar as an IndexValue. Rejects bool (a Python int subclass)."""
    if isinstance(raw, bool):
        raise NdsqError(Reason.INVALID_JSON, f"index value must be an integer, got bool {raw!r}")
    if isinstance(raw, int):
        return raw
    if raw == "-inf" or raw == "+inf":
        return raw  # type: ignore[return-value]
    raise NdsqError(Reason.INVALID_JSON, f"invalid index value: {raw!r}")


@dataclass(frozen=True)
class ImplicitValue:
    """An index bound plus an explicit/implicit flag.

    JSON: a bare value is explicit (`7`, `"-inf"`); the same wrapped in a
    one-element array is implicit (`[7]`, `["-inf"]`).
    """

    value: IndexValue
    implicit: bool

    @classmethod
    def explicit(cls, value: IndexValue) -> "ImplicitValue":
        return cls(value=value, implicit=False)

    @classmethod
    def from_json(cls, raw: object) -> "ImplicitValue":
        if isinstance(raw, list):
            if len(raw) != 1:
                raise NdsqError(Reason.INVALID_JSON, "implicit bound must be a 1-element array")
            return cls(value=parse_index_value(raw[0]), implicit=True)
        return cls(value=parse_index_value(raw), implicit=False)

    def to_json(self) -> object:
        return [self.value] if self.implicit else self.value
```

- [ ] **Step 4: Run test to verify it passes**

Run: `uv run pytest tests/test_values.py -q`
Expected: 7 passed.

- [ ] **Step 5: Commit**

```bash
git add python/ndsq/src/ndsq/values.py python/ndsq/tests/test_values.py
git commit -m "feat(py): IndexValue and ImplicitValue with [n]-bracket convention"
```

---

## Task 4: `domain.py` — Domain and canonicalize_domain

**Files:**
- Create: `python/ndsq/src/ndsq/domain.py`
- Test: `python/ndsq/tests/test_domain.py`

`canonicalize_domain` takes the raw (JSON-level) optional bound arrays and produces a canonical `Domain` (inclusive_min + exclusive_max), mirroring the Rust `RawDomain::into_domain`.

- [ ] **Step 1: Write the failing test**

Create `python/ndsq/tests/test_domain.py`:

```python
import pytest

from ndsq.domain import Domain, canonicalize_domain
from ndsq.errors import NdsqError, Reason
from ndsq.values import ImplicitValue


def fin(n: int) -> ImplicitValue:
    return ImplicitValue.explicit(n)


def test_shape_only_defaults_min_to_zero():
    d = canonicalize_domain(shape=[3, 4])
    assert d.rank == 2
    assert d.inclusive_min == [fin(0), fin(0)]
    assert d.exclusive_max == [fin(3), fin(4)]
    assert d.labels == ["", ""]


def test_inclusive_max_converts_to_exclusive():
    d = canonicalize_domain(inclusive_min=[0], inclusive_max=[9])
    assert d.exclusive_max == [fin(10)]


def test_implicit_and_infinite_bounds_preserved():
    d = canonicalize_domain(inclusive_min=[["-inf"], 0], exclusive_max=[["+inf"], 4])
    assert d.inclusive_min == [ImplicitValue("-inf", True), fin(0)]
    assert d.exclusive_max == [ImplicitValue("+inf", True), fin(4)]


def test_multiple_upper_bounds_is_error():
    with pytest.raises(NdsqError) as e:
        canonicalize_domain(shape=[3], exclusive_max=[3])
    assert e.value.reason is Reason.MULTIPLE_UPPER_BOUNDS


def test_length_disagreement_is_rank_mismatch():
    with pytest.raises(NdsqError) as e:
        canonicalize_domain(inclusive_min=[0, 0], shape=[3])
    assert e.value.reason is Reason.RANK_MISMATCH


def test_rank_field_disagreement_is_rank_mismatch():
    with pytest.raises(NdsqError) as e:
        canonicalize_domain(rank=2, inclusive_min=[0])
    assert e.value.reason is Reason.RANK_MISMATCH


def test_domain_to_json_fields():
    d = canonicalize_domain(inclusive_min=[0, 0], exclusive_max=[3, 4])
    assert d.to_json_fields() == {
        "input_rank": 2,
        "input_inclusive_min": [0, 0],
        "input_exclusive_max": [3, 4],
        "input_labels": ["", ""],
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run pytest tests/test_domain.py -q`
Expected: FAIL — `No module named 'ndsq.domain'`.

- [ ] **Step 3: Write the implementation**

Create `python/ndsq/src/ndsq/domain.py`:

```python
"""The canonical input domain and its construction from raw JSON fields."""

from __future__ import annotations

from dataclasses import dataclass

from .errors import NdsqError, Reason
from .values import ImplicitValue


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
    imin = [ImplicitValue.from_json(b) for b in inclusive_min] if inclusive_min is not None else None
    emax = [ImplicitValue.from_json(b) for b in exclusive_max] if exclusive_max is not None else None
    incmax = [ImplicitValue.from_json(b) for b in inclusive_max] if inclusive_max is not None else None
    shp = [ImplicitValue.from_json(b) for b in shape] if shape is not None else None

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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `uv run pytest tests/test_domain.py -q`
Expected: 7 passed.

- [ ] **Step 5: Commit**

```bash
git add python/ndsq/src/ndsq/domain.py python/ndsq/tests/test_domain.py
git commit -m "feat(py): Domain and canonicalize_domain (upper-bound canonicalization)"
```

---

## Task 5: `output.py` — output-map dataclasses

**Files:**
- Create: `python/ndsq/src/ndsq/output.py`
- Test: `python/ndsq/tests/test_output.py`

Three output-map kinds as dataclasses, plus `canonicalize_output_map(raw_dict)` (default-filling + discrimination) and `output_map_to_json`.

- [ ] **Step 1: Write the failing test**

Create `python/ndsq/tests/test_output.py`:

```python
from ndsq.output import (
    ConstantMap,
    IndexArrayMap,
    SingleInputDimension,
    canonicalize_output_map,
    output_map_to_json,
)


def test_constant_map_carries_only_offset():
    m = canonicalize_output_map({"offset": 3})
    assert m == ConstantMap(offset=3)
    assert output_map_to_json(m) == {"offset": 3}


def test_single_input_dimension_fills_defaults():
    m = canonicalize_output_map({"input_dimension": 2})
    assert m == SingleInputDimension(offset=0, stride=1, input_dimension=2)
    assert output_map_to_json(m) == {"offset": 0, "stride": 1, "input_dimension": 2}


def test_index_array_fills_offset_stride_bounds():
    m = canonicalize_output_map({"index_array": [1, 2, 3]})
    assert m == IndexArrayMap(offset=0, stride=1, index_array=[1, 2, 3], bounds=("-inf", "+inf"))
    assert output_map_to_json(m) == {
        "offset": 0,
        "stride": 1,
        "index_array": [1, 2, 3],
        "index_array_bounds": ["-inf", "+inf"],
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run pytest tests/test_output.py -q`
Expected: FAIL — `No module named 'ndsq.output'`.

- [ ] **Step 3: Write the implementation**

Create `python/ndsq/src/ndsq/output.py`:

```python
"""The three output-map kinds (canonical form) and their JSON encoding."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Union

from .values import IndexValue, parse_index_value


@dataclass
class ConstantMap:
    offset: int


@dataclass
class SingleInputDimension:
    offset: int
    stride: int
    input_dimension: int


@dataclass
class IndexArrayMap:
    offset: int
    stride: int
    index_array: Any  # raw nested JSON; deep validation deferred
    bounds: tuple[IndexValue, IndexValue]


OutputMap = Union[ConstantMap, SingleInputDimension, IndexArrayMap]


def canonicalize_output_map(raw: dict) -> OutputMap:
    """Default-fill and discriminate a raw output-map object."""
    offset = raw.get("offset", 0)
    stride = raw.get("stride", 1)
    if "index_array" in raw:
        b = raw.get("index_array_bounds", ["-inf", "+inf"])
        bounds = (parse_index_value(b[0]), parse_index_value(b[1]))
        return IndexArrayMap(offset=offset, stride=stride, index_array=raw["index_array"], bounds=bounds)
    if "input_dimension" in raw:
        return SingleInputDimension(offset=offset, stride=stride, input_dimension=raw["input_dimension"])
    return ConstantMap(offset=offset)


def output_map_to_json(m: OutputMap) -> dict:
    if isinstance(m, ConstantMap):
        return {"offset": m.offset}
    if isinstance(m, SingleInputDimension):
        return {"offset": m.offset, "stride": m.stride, "input_dimension": m.input_dimension}
    # IndexArrayMap
    return {
        "offset": m.offset,
        "stride": m.stride,
        "index_array": m.index_array,
        "index_array_bounds": [m.bounds[0], m.bounds[1]],
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `uv run pytest tests/test_output.py -q`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add python/ndsq/src/ndsq/output.py python/ndsq/tests/test_output.py
git commit -m "feat(py): output-map dataclasses with default filling and JSON encoding"
```

---

## Task 6: `transform.py` — Transform and canonicalize_transform

**Files:**
- Create: `python/ndsq/src/ndsq/transform.py`
- Test: `python/ndsq/tests/test_transform.py`

- [ ] **Step 1: Write the failing test**

Create `python/ndsq/tests/test_transform.py`:

```python
from ndsq.transform import Transform, canonicalize_transform


def test_omitted_output_becomes_explicit_identity():
    t = canonicalize_transform(
        {"input_inclusive_min": [0, 0], "input_exclusive_max": [3, 4]}
    )
    d = t.to_dict()
    assert d["input_inclusive_min"] == [0, 0]
    assert d["input_exclusive_max"] == [3, 4]
    assert d["output"] == [
        {"offset": 0, "stride": 1, "input_dimension": 0},
        {"offset": 0, "stride": 1, "input_dimension": 1},
    ]


def test_canonicalize_is_idempotent():
    once = canonicalize_transform({"input_inclusive_min": [0], "input_shape": [5]})
    twice = canonicalize_transform(once.to_dict())
    assert once.to_dict() == twice.to_dict()


def test_explicit_output_all_three_kinds():
    t = canonicalize_transform(
        {
            "input_inclusive_min": [0],
            "input_exclusive_max": [3],
            "output": [
                {"offset": 7},
                {"input_dimension": 0, "stride": 2},
                {"index_array": [1, 2, 3]},
            ],
        }
    )
    assert t.to_dict()["output"] == [
        {"offset": 7},
        {"offset": 0, "stride": 2, "input_dimension": 0},
        {"offset": 0, "stride": 1, "index_array": [1, 2, 3], "index_array_bounds": ["-inf", "+inf"]},
    ]


def test_isinstance():
    assert isinstance(canonicalize_transform({"input_shape": [1]}), Transform)
```

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run pytest tests/test_transform.py -q`
Expected: FAIL — `No module named 'ndsq.transform'`.

- [ ] **Step 3: Write the implementation**

Create `python/ndsq/src/ndsq/transform.py`:

```python
"""The canonical core: a Transform (domain + explicit output maps)."""

from __future__ import annotations

from dataclasses import dataclass

from .domain import Domain, canonicalize_domain
from .output import OutputMap, SingleInputDimension, canonicalize_output_map, output_map_to_json


@dataclass
class Transform:
    """The canonical core. Serialize with `to_dict()` (the bare transform body, no `kind`)."""

    domain: Domain
    output: list[OutputMap]

    def to_dict(self) -> dict:
        body = self.domain.to_json_fields()
        body["output"] = [output_map_to_json(m) for m in self.output]
        return body


def identity_output(rank: int) -> list[OutputMap]:
    return [SingleInputDimension(offset=0, stride=1, input_dimension=k) for k in range(rank)]


def canonicalize_transform(msg: dict) -> Transform:
    """Canonicalize a `transform` message body (uses the input_-prefixed field names)."""
    domain = canonicalize_domain(
        rank=msg.get("input_rank"),
        inclusive_min=msg.get("input_inclusive_min"),
        exclusive_max=msg.get("input_exclusive_max"),
        inclusive_max=msg.get("input_inclusive_max"),
        shape=msg.get("input_shape"),
        labels=msg.get("input_labels"),
    )
    raw_output = msg.get("output")
    if raw_output is not None:
        output = [canonicalize_output_map(m) for m in raw_output]
    else:
        output = identity_output(domain.rank)
    return Transform(domain=domain, output=output)
```

- [ ] **Step 4: Run test to verify it passes**

Run: `uv run pytest tests/test_transform.py -q`
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add python/ndsq/src/ndsq/transform.py python/ndsq/tests/test_transform.py
git commit -m "feat(py): Transform and canonicalize_transform (explicit identity output)"
```

---

## Task 7: `messages.py` + `builders.py` — TypedDicts and builder dataclasses

**Files:**
- Create: `python/ndsq/src/ndsq/messages.py`, `python/ndsq/src/ndsq/builders.py`
- Test: `python/ndsq/tests/test_builders.py`

TypedDicts type the JSON message layer; builder dataclasses give ergonomic construction (`.to_message()` → the TypedDict).

- [ ] **Step 1: Write the failing test**

Create `python/ndsq/tests/test_builders.py`:

```python
from ndsq.builders import Box, Point, Points, Slice


def test_point_to_message():
    assert Point(coords=[4, 7]).to_message() == {"kind": "point", "coords": [4, 7]}


def test_box_omits_unset_fields():
    assert Box(inclusive_min=[0, 0], exclusive_max=[3, 4]).to_message() == {
        "kind": "box",
        "inclusive_min": [0, 0],
        "exclusive_max": [3, 4],
    }
    assert Box(shape=[5]).to_message() == {"kind": "box", "shape": [5]}


def test_slice_omits_unset_step_and_labels():
    assert Slice(start=[0], stop=[10]).to_message() == {
        "kind": "slice",
        "start": [0],
        "stop": [10],
    }
    assert Slice(start=[0], stop=[10], step=[2]).to_message() == {
        "kind": "slice",
        "start": [0],
        "stop": [10],
        "step": [2],
    }


def test_points_to_message():
    assert Points(coords=[[1, 10], [2, 20]]).to_message() == {
        "kind": "points",
        "coords": [[1, 10], [2, 20]],
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run pytest tests/test_builders.py -q`
Expected: FAIL — `No module named 'ndsq.builders'`.

- [ ] **Step 3: Write `messages.py`**

Create `python/ndsq/src/ndsq/messages.py`:

```python
"""TypedDict models of the JSON message shapes (the on-the-wire layer)."""

from __future__ import annotations

from typing import Any, Literal, NotRequired, TypedDict, Union

# A JSON-level bound: an int, a sentinel string, or one of those wrapped in a
# one-element list (implicit). Typed loosely; semantic validation is in values.py.
BoundJson = Union[int, str, list]


class PointMessage(TypedDict):
    kind: Literal["point"]
    coords: list[int]


class BoxMessage(TypedDict):
    kind: Literal["box"]
    inclusive_min: NotRequired[list[BoundJson]]
    exclusive_max: NotRequired[list[BoundJson]]
    inclusive_max: NotRequired[list[BoundJson]]
    shape: NotRequired[list[BoundJson]]
    labels: NotRequired[list[str]]


class SliceMessage(TypedDict):
    kind: Literal["slice"]
    start: list[int]
    stop: list[int]
    step: NotRequired[list[int]]
    labels: NotRequired[list[str]]


class PointsMessage(TypedDict):
    kind: Literal["points"]
    coords: list[list[int]]


class OutputMapDict(TypedDict):
    offset: NotRequired[int]
    stride: NotRequired[int]
    input_dimension: NotRequired[int]
    index_array: NotRequired[Any]
    index_array_bounds: NotRequired[list]


class TransformMessage(TypedDict):
    kind: Literal["transform"]
    input_rank: NotRequired[int]
    input_inclusive_min: NotRequired[list[BoundJson]]
    input_exclusive_max: NotRequired[list[BoundJson]]
    input_inclusive_max: NotRequired[list[BoundJson]]
    input_shape: NotRequired[list[BoundJson]]
    input_labels: NotRequired[list[str]]
    output: NotRequired[list[OutputMapDict]]


Message = Union[PointMessage, BoxMessage, SliceMessage, PointsMessage, TransformMessage]
```

- [ ] **Step 4: Write `builders.py`**

Create `python/ndsq/src/ndsq/builders.py`:

```python
"""Ergonomic dataclass builders for the four shorthand message kinds."""

from __future__ import annotations

from dataclasses import dataclass

from .messages import BoxMessage, PointMessage, PointsMessage, SliceMessage


@dataclass
class Point:
    coords: list[int]

    def to_message(self) -> PointMessage:
        return {"kind": "point", "coords": self.coords}


@dataclass
class Box:
    inclusive_min: list | None = None
    exclusive_max: list | None = None
    inclusive_max: list | None = None
    shape: list | None = None
    labels: list[str] | None = None

    def to_message(self) -> BoxMessage:
        msg: dict = {"kind": "box"}
        for field in ("inclusive_min", "exclusive_max", "inclusive_max", "shape", "labels"):
            value = getattr(self, field)
            if value is not None:
                msg[field] = value
        return msg  # type: ignore[return-value]


@dataclass
class Slice:
    start: list[int]
    stop: list[int]
    step: list[int] | None = None
    labels: list[str] | None = None

    def to_message(self) -> SliceMessage:
        msg: dict = {"kind": "slice", "start": self.start, "stop": self.stop}
        if self.step is not None:
            msg["step"] = self.step
        if self.labels is not None:
            msg["labels"] = self.labels
        return msg  # type: ignore[return-value]


@dataclass
class Points:
    coords: list[list[int]]

    def to_message(self) -> PointsMessage:
        return {"kind": "points", "coords": self.coords}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `uv run pytest tests/test_builders.py -q`
Expected: 4 passed.

- [ ] **Step 6: Commit**

```bash
git add python/ndsq/src/ndsq/messages.py python/ndsq/src/ndsq/builders.py python/ndsq/tests/test_builders.py
git commit -m "feat(py): TypedDict messages and builder dataclasses"
```

---

## Task 8: `shorthand.py` (point, box) + `__init__` API (parse, normalize)

**Files:**
- Create: `python/ndsq/src/ndsq/shorthand.py`
- Modify: `python/ndsq/src/ndsq/__init__.py`
- Test: `python/ndsq/tests/test_shorthand_point_box.py`

- [ ] **Step 1: Write the failing test**

Create `python/ndsq/tests/test_shorthand_point_box.py`:

```python
from ndsq import normalize, parse
from ndsq.builders import Box


def norm(text: str) -> dict:
    return normalize(parse(text)).to_dict()


def test_point_desugars_to_constant_maps():
    d = norm('{"kind": "point", "coords": [4, 7]}')
    assert d["input_rank"] == 0
    assert d["input_inclusive_min"] == []
    assert d["input_exclusive_max"] == []
    assert d["output"] == [{"offset": 4}, {"offset": 7}]


def test_box_desugars_to_identity():
    d = norm('{"kind": "box", "inclusive_min": [0, 0], "exclusive_max": [3, 4]}')
    assert d["output"] == [
        {"offset": 0, "stride": 1, "input_dimension": 0},
        {"offset": 0, "stride": 1, "input_dimension": 1},
    ]


def test_box_shape_only_defaults_origin_zero():
    d = norm('{"kind": "box", "shape": [5]}')
    assert d["input_inclusive_min"] == [0]
    assert d["input_exclusive_max"] == [5]


def test_normalize_accepts_a_builder():
    d = normalize(Box(shape=[5])).to_dict()
    assert d["input_exclusive_max"] == [5]
```

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run pytest tests/test_shorthand_point_box.py -q`
Expected: FAIL — `cannot import name 'normalize' from 'ndsq'`.

- [ ] **Step 3: Write `shorthand.py` (point + box; slice/points raise NotImplementedError for now)**

Create `python/ndsq/src/ndsq/shorthand.py`:

```python
"""Desugaring of the four shorthand kinds to a canonical Transform."""

from __future__ import annotations

from .domain import Domain, canonicalize_domain
from .errors import NdsqError, Reason
from .output import ConstantMap, IndexArrayMap, SingleInputDimension
from .transform import Transform
from .values import ImplicitValue


def desugar_point(msg: dict) -> Transform:
    coords = msg["coords"]
    domain = Domain(rank=0, inclusive_min=[], exclusive_max=[], labels=[])
    output = [ConstantMap(offset=c) for c in coords]
    return Transform(domain=domain, output=output)


def desugar_box(msg: dict) -> Transform:
    domain = canonicalize_domain(
        inclusive_min=msg.get("inclusive_min"),
        exclusive_max=msg.get("exclusive_max"),
        inclusive_max=msg.get("inclusive_max"),
        shape=msg.get("shape"),
        labels=msg.get("labels"),
    )
    output = [SingleInputDimension(offset=0, stride=1, input_dimension=k) for k in range(domain.rank)]
    return Transform(domain=domain, output=output)


def desugar_slice(msg: dict) -> Transform:  # implemented in Task 9
    raise NotImplementedError("desugar_slice — Task 9")


def desugar_points(msg: dict) -> Transform:  # implemented in Task 10
    raise NotImplementedError("desugar_points — Task 10")
```

- [ ] **Step 4: Write the public API in `__init__.py`**

Replace `python/ndsq/src/ndsq/__init__.py` with:

```python
"""ndsq — JSON-serializable n-dimensional spatial queries.

Parse a JSON string with `parse`, then reduce it to a canonical `Transform`
with `normalize`. `normalize` also accepts a builder dataclass directly.
"""

from __future__ import annotations

import json

from .builders import Box, Point, Points, Slice
from .domain import Domain
from .errors import NdsqError, Reason
from .messages import Message
from .output import ConstantMap, IndexArrayMap, OutputMap, SingleInputDimension
from .shorthand import desugar_box, desugar_point, desugar_points, desugar_slice
from .transform import Transform, canonicalize_transform
from .values import ImplicitValue, IndexValue

__all__ = [
    "parse",
    "normalize",
    "Transform",
    "Domain",
    "OutputMap",
    "ConstantMap",
    "SingleInputDimension",
    "IndexArrayMap",
    "ImplicitValue",
    "IndexValue",
    "Message",
    "Point",
    "Box",
    "Slice",
    "Points",
    "NdsqError",
    "Reason",
]

_KNOWN_KINDS = {"point", "box", "slice", "points", "transform"}
_BUILDERS = (Point, Box, Slice, Points)


def parse(text: str) -> Message:
    """Parse a JSON string into a validated Message (dispatching on `kind`)."""
    try:
        obj = json.loads(text)
    except (json.JSONDecodeError, TypeError) as exc:
        raise NdsqError(Reason.INVALID_JSON, str(exc)) from exc
    if not isinstance(obj, dict):
        raise NdsqError(Reason.INVALID_JSON, "message must be a JSON object")
    kind = obj.get("kind")
    if not isinstance(kind, str):
        raise NdsqError(Reason.INVALID_JSON, "missing string `kind`")
    if kind not in _KNOWN_KINDS:
        raise NdsqError(Reason.UNKNOWN_KIND, f"unknown kind: {kind}")
    return obj  # type: ignore[return-value]


def normalize(message: Message | Point | Box | Slice | Points) -> Transform:
    """Reduce a message (or a builder dataclass) to its canonical Transform."""
    if isinstance(message, _BUILDERS):
        message = message.to_message()
    kind = message["kind"]
    if kind == "point":
        return desugar_point(message)
    if kind == "box":
        return desugar_box(message)
    if kind == "slice":
        return desugar_slice(message)
    if kind == "points":
        return desugar_points(message)
    if kind == "transform":
        return canonicalize_transform(message)
    raise NdsqError(Reason.UNKNOWN_KIND, f"unknown kind: {kind}")
```

- [ ] **Step 5: Run test to verify it passes**

Run: `uv run pytest tests/test_shorthand_point_box.py -q`
Expected: 4 passed.

- [ ] **Step 6: Commit**

```bash
git add python/ndsq/src/ndsq/shorthand.py python/ndsq/src/ndsq/__init__.py python/ndsq/tests/test_shorthand_point_box.py
git commit -m "feat(py): desugar point and box; parse/normalize public API"
```

---

## Task 9: `shorthand.py` — slice desugaring

**Files:**
- Modify: `python/ndsq/src/ndsq/shorthand.py`
- Test: `python/ndsq/tests/test_shorthand_slice.py`

Implements spec §5.3 for `s > 0`: `m = max(0, ceil((b-a)/s))`, `o = a // s` (Python `//` is floor for positive `s`), `offset = a - s*o`, map `single_input_dimension(k, offset, s)`, domain `[o, o+m)`. `s == 0` → `step_zero`; `s < 0` → `negative_step_unsupported`.

- [ ] **Step 1: Write the failing test**

Create `python/ndsq/tests/test_shorthand_slice.py`:

```python
import pytest

from ndsq import normalize, parse
from ndsq.errors import NdsqError, Reason


def norm(text: str) -> dict:
    return normalize(parse(text)).to_dict()


def err(text: str) -> Reason:
    with pytest.raises(NdsqError) as e:
        normalize(parse(text))
    return e.value.reason


def test_unit_step_preserves_source_frame():
    d = norm('{"kind": "slice", "start": [5], "stop": [10]}')
    assert d["input_inclusive_min"] == [5]
    assert d["input_exclusive_max"] == [10]
    assert d["output"] == [{"offset": 0, "stride": 1, "input_dimension": 0}]


def test_divisible_strided_slice():
    d = norm('{"kind": "slice", "start": [4], "stop": [10], "step": [2]}')
    assert d["input_inclusive_min"] == [2]
    assert d["input_exclusive_max"] == [5]
    assert d["output"] == [{"offset": 0, "stride": 2, "input_dimension": 0}]


def test_nondivisible_strided_slice_phase_offset():
    d = norm('{"kind": "slice", "start": [5], "stop": [10], "step": [2]}')
    assert d["input_inclusive_min"] == [2]
    assert d["input_exclusive_max"] == [5]
    assert d["output"] == [{"offset": 1, "stride": 2, "input_dimension": 0}]


def test_empty_slice_zero_length():
    d = norm('{"kind": "slice", "start": [10], "stop": [10]}')
    assert d["input_inclusive_min"] == [10]
    assert d["input_exclusive_max"] == [10]


def test_2d_mixed_step():
    d = norm('{"kind": "slice", "start": [0, 5], "stop": [10, 10], "step": [2, 1]}')
    assert d["input_inclusive_min"] == [0, 5]
    assert d["input_exclusive_max"] == [5, 10]


def test_errors():
    assert err('{"kind": "slice", "start": [0], "stop": [4], "step": [0]}') is Reason.STEP_ZERO
    assert err('{"kind": "slice", "start": [9], "stop": [0], "step": [-2]}') is Reason.NEGATIVE_STEP_UNSUPPORTED
    assert err('{"kind": "slice", "start": [0, 0], "stop": [4]}') is Reason.RANK_MISMATCH
```

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run pytest tests/test_shorthand_slice.py -q`
Expected: FAIL — `NotImplementedError: desugar_slice — Task 9`.

- [ ] **Step 3: Write the implementation**

Replace the `desugar_slice` stub in `python/ndsq/src/ndsq/shorthand.py` with:

```python
def _ceil_div(p: int, q: int) -> int:
    """Ceiling of p/q for p >= 0, q > 0."""
    return (p + q - 1) // q


def desugar_slice(msg: dict) -> Transform:
    start = msg["start"]
    stop = msg["stop"]
    rank = len(start)
    if len(stop) != rank:
        raise NdsqError(Reason.RANK_MISMATCH, "start and stop must have equal length")
    step = msg.get("step")
    if step is None:
        step = [1] * rank
    elif len(step) != rank:
        raise NdsqError(Reason.RANK_MISMATCH, "step length must match start/stop")
    labels = msg.get("labels")
    if labels is not None and len(labels) != rank:
        raise NdsqError(Reason.RANK_MISMATCH, "labels length must match start/stop")

    inclusive_min: list[ImplicitValue] = []
    exclusive_max: list[ImplicitValue] = []
    output: list = []
    for k in range(rank):
        a, b, s = start[k], stop[k], step[k]
        if s == 0:
            raise NdsqError(Reason.STEP_ZERO, "step must be non-zero")
        if s < 0:
            raise NdsqError(Reason.NEGATIVE_STEP_UNSUPPORTED, "negative step is not yet specified")
        m = 0 if b <= a else _ceil_div(b - a, s)
        o = a // s  # floor(a/s) for s > 0
        offset = a - s * o  # lattice phase in [0, s)
        inclusive_min.append(ImplicitValue.explicit(o))
        exclusive_max.append(ImplicitValue.explicit(o + m))
        output.append(SingleInputDimension(offset=offset, stride=s, input_dimension=k))

    out_labels = labels if labels is not None else ["" for _ in range(rank)]
    domain = Domain(rank=rank, inclusive_min=inclusive_min, exclusive_max=exclusive_max, labels=out_labels)
    return Transform(domain=domain, output=output)
```

- [ ] **Step 4: Run test to verify it passes**

Run: `uv run pytest tests/test_shorthand_slice.py -q`
Expected: 6 passed.

- [ ] **Step 5: Commit**

```bash
git add python/ndsq/src/ndsq/shorthand.py python/ndsq/tests/test_shorthand_slice.py
git commit -m "feat(py): desugar slice (positive step, coordinate-preserving)"
```

---

## Task 10: `shorthand.py` — points desugaring

**Files:**
- Modify: `python/ndsq/src/ndsq/shorthand.py`
- Test: `python/ndsq/tests/test_shorthand_points.py`

Implements spec §5.4: `m` points of rank `n` → `input_rank: 1`, domain `[0, m)`, `n` index-array maps (the row-major list transposed to columns). Ragged points → `rank_mismatch`.

- [ ] **Step 1: Write the failing test**

Create `python/ndsq/tests/test_shorthand_points.py`:

```python
import pytest

from ndsq import normalize, parse
from ndsq.errors import NdsqError, Reason


def norm(text: str) -> dict:
    return normalize(parse(text)).to_dict()


def test_points_transpose_to_columnar():
    d = norm('{"kind": "points", "coords": [[1, 10], [2, 20], [3, 30]]}')
    assert d["input_rank"] == 1
    assert d["input_inclusive_min"] == [0]
    assert d["input_exclusive_max"] == [3]
    assert d["output"] == [
        {"offset": 0, "stride": 1, "index_array": [1, 2, 3], "index_array_bounds": ["-inf", "+inf"]},
        {"offset": 0, "stride": 1, "index_array": [10, 20, 30], "index_array_bounds": ["-inf", "+inf"]},
    ]


def test_empty_points():
    d = norm('{"kind": "points", "coords": []}')
    assert d["input_rank"] == 1
    assert d["input_exclusive_max"] == [0]
    assert d["output"] == []


def test_ragged_points_is_rank_mismatch():
    with pytest.raises(NdsqError) as e:
        normalize(parse('{"kind": "points", "coords": [[1, 2], [3]]}'))
    assert e.value.reason is Reason.RANK_MISMATCH
```

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run pytest tests/test_shorthand_points.py -q`
Expected: FAIL — `NotImplementedError: desugar_points — Task 10`.

- [ ] **Step 3: Write the implementation**

Replace the `desugar_points` stub in `python/ndsq/src/ndsq/shorthand.py` with:

```python
def desugar_points(msg: dict) -> Transform:
    coords = msg["coords"]
    m = len(coords)
    n = len(coords[0]) if coords else 0
    for point in coords:
        if len(point) != n:
            raise NdsqError(Reason.RANK_MISMATCH, "all points must have equal dimensionality")

    domain = Domain(
        rank=1,
        inclusive_min=[ImplicitValue.explicit(0)],
        exclusive_max=[ImplicitValue.explicit(m)],
        labels=[""],
    )
    output: list = []
    for k in range(n):
        column = [coords[i][k] for i in range(m)]
        output.append(IndexArrayMap(offset=0, stride=1, index_array=column, bounds=("-inf", "+inf")))
    return Transform(domain=domain, output=output)
```

- [ ] **Step 4: Run test to verify it passes**

Run: `uv run pytest tests/test_shorthand_points.py -q`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add python/ndsq/src/ndsq/shorthand.py python/ndsq/tests/test_shorthand_points.py
git commit -m "feat(py): desugar points (row-major to columnar index arrays)"
```

---

## Task 11: parse / kind-dispatch tests

**Files:**
- Test: `python/ndsq/tests/test_parse.py`

`parse`/`normalize` already implement this (Task 8). These lock the behavior in.

- [ ] **Step 1: Write the tests**

Create `python/ndsq/tests/test_parse.py`:

```python
import pytest

from ndsq import normalize, parse
from ndsq.errors import NdsqError, Reason


def test_unknown_kind():
    with pytest.raises(NdsqError) as e:
        parse('{"kind": "bogus"}')
    assert e.value.reason is Reason.UNKNOWN_KIND


def test_missing_kind_is_invalid_json():
    with pytest.raises(NdsqError) as e:
        parse('{"coords": [1]}')
    assert e.value.reason is Reason.INVALID_JSON


def test_malformed_json_is_invalid_json():
    with pytest.raises(NdsqError) as e:
        parse("{ not json")
    assert e.value.reason is Reason.INVALID_JSON


def test_non_object_is_invalid_json():
    with pytest.raises(NdsqError) as e:
        parse("[1, 2, 3]")
    assert e.value.reason is Reason.INVALID_JSON


def test_parse_then_normalize_roundtrip():
    t = normalize(parse('{"kind": "box", "shape": [2, 3]}'))
    assert t.to_dict()["input_exclusive_max"] == [2, 3]
```

- [ ] **Step 2: Run the tests**

Run: `uv run pytest tests/test_parse.py -q`
Expected: 5 passed (behavior implemented in Task 8). If any fail, reconcile `parse`/`normalize` with Task 8 rather than weakening the test.

- [ ] **Step 3: Run the whole unit suite**

Run: `uv run pytest -q`
Expected: all unit tests across Tasks 2–11 pass.

- [ ] **Step 4: Commit**

```bash
git add python/ndsq/tests/test_parse.py
git commit -m "test(py): parse and kind-dispatch regression tests"
```

---

## Task 12: Conformance runner

**Files:**
- Create: `python/ndsq/tests/test_conformance.py`

Runs the **same** `/conformance/` corpus the Rust reference passes: schema-validate every success input, normalize, compare `to_dict()` to the fixture's `normalized` (structural equality); for error cases, assert `NdsqError` with the stated reason code.

- [ ] **Step 1: Write the runner**

Create `python/ndsq/tests/test_conformance.py`:

```python
"""Runs the repo-root /conformance corpus against the Python implementation."""

from __future__ import annotations

import glob
import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

from ndsq import NdsqError, normalize, parse

REPO_ROOT = Path(__file__).parents[3]
CORPUS_DIR = REPO_ROOT / "conformance"
SCHEMA_PATH = REPO_ROOT / "schema" / "ndsq.schema.json"


def _load_cases() -> list[tuple[str, dict]]:
    cases: list[tuple[str, dict]] = []
    for path in sorted(glob.glob(str(CORPUS_DIR / "*.json"))):
        for case in json.loads(Path(path).read_text()):
            cases.append((case.get("name", "<unnamed>"), case))
    return cases


_VALIDATOR = Draft202012Validator(json.loads(SCHEMA_PATH.read_text()))
_CASES = _load_cases()


def test_corpus_is_populated():
    assert len(_CASES) >= 15, f"expected a populated corpus, found {len(_CASES)}"


@pytest.mark.parametrize("name,case", _CASES, ids=[c[0] for c in _CASES])
def test_corpus_case(name: str, case: dict):
    input_msg = case["input"]
    if "error" in case:
        # Error inputs may be intentionally schema-invalid; not schema-checked.
        with pytest.raises(NdsqError) as exc:
            normalize(parse(json.dumps(input_msg)))
        assert exc.value.reason.value == case["error"]
    else:
        assert _VALIDATOR.is_valid(input_msg), f"{name}: input fails schema"
        got = normalize(parse(json.dumps(input_msg))).to_dict()
        assert got == case["normalized"]
```

- [ ] **Step 2: Run the runner**

Run: `uv run pytest tests/test_conformance.py -q`
Expected: PASS — one parametrized case per fixture (24 currently), all green. If a success case shows a `normalized` mismatch, it is a **Python bug** — fix the Python, never the fixture.

- [ ] **Step 3: Run the full suite**

Run: `uv run pytest -q`
Expected: all unit tests + every conformance case pass.

- [ ] **Step 4: Commit**

```bash
git add python/ndsq/tests/test_conformance.py
git commit -m "test(py): conformance runner against the shared corpus"
```

---

## Task 13: README and final verification

**Files:**
- Modify: `python/ndsq/README.md`

- [ ] **Step 1: Write the package README**

Replace `python/ndsq/README.md` with: one paragraph on what the package is (the Python peer implementation of ndsq, passing the shared conformance corpus); the two-layer model (TypedDict messages + dataclass builders/results); a short usage example:

```python
from ndsq import normalize, parse, Box

normalize(parse('{"kind": "slice", "start": [0], "stop": [10], "step": [2]}')).to_dict()
normalize(Box(inclusive_min=[0, 0], exclusive_max=[3, 4])).to_dict()
```

and a note that it adapts tensorstore's index model (link `../../spec/ndsq.md`) and is validated against `../../conformance/`.

- [ ] **Step 2: Full verification**

Run: `uv run pytest -q`
Expected: every test passes (unit + all conformance cases).

Run: `uv run python -c "from ndsq import normalize, Box; print(normalize(Box(shape=[3])).to_dict())"`
Expected: prints the canonical transform dict for a `[0,3)` box.

- [ ] **Step 3: Commit**

```bash
git add python/ndsq/README.md
git commit -m "docs(py): package README with usage and pointers"
```

---

## Self-review checklist (run after implementation)

- **Spec coverage:** every desugaring has a task and is exercised by the shared corpus (Task 12): point/box=Task 8, slice=Task 9, points=Task 10, transform=Task 6. Errors: Reason codes=Task 2; step/negative/rank in slice=Task 9; multiple_upper_bounds/rank in domain=Task 4; unknown_kind/invalid_json in parse=Task 8/11.
- **Contract parity:** the Python normalizes to the SAME `normalized` bodies as Rust (same corpus, Task 12). `box` accepts implicit/inf bounds (Task 4 test `test_implicit_and_infinite_bounds_preserved`); `slice` integer-only.
- **Type consistency:** `parse`, `normalize`, `canonicalize_domain`, `canonicalize_transform`, `canonicalize_output_map`, `output_map_to_json`, `Transform.to_dict`, `Domain.to_json_fields`, `ImplicitValue.from_json/to_json/explicit`, `desugar_*` are used with identical signatures across tasks.
- **No placeholders:** the only `NotImplementedError`s are the Task 8 stubs, replaced in Tasks 9–10.
- **Python gotchas covered:** `bool` rejected as IndexValue (Task 3); `//` floor division relied on for positive step (Task 9); dict `==` is structural so output key order is irrelevant (Task 12).
