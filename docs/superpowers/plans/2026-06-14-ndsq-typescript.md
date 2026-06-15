# ndsq TypeScript Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An idiomatic TypeScript peer implementation of ndsq that passes the same `/conformance/` corpus (29 fixtures) as the Rust and Python references.

**Architecture:** Hybrid casing, zarrita.js-style. **snake_case wire types** mirror the JSON byte-for-byte (`Message` discriminated union, `Transform`, output maps) so `parse`/`normalize` are zero-conversion and faithful to the corpus. **camelCase builder functions** (`box({inputInclusiveMin})`, `slice(...)`, …) give idiomatic construction, emitting snake_case message objects. `normalize(message) → Transform` returns the canonical body as a plain object (the JSON itself — no separate serializer). Runtime deps: **zero**.

**Tech Stack:** Node 24 (runs `.ts` natively via type-stripping — no build step for tests), `node:test` + `node:assert` (built-in test runner), `ajv` (dev-only) for schema validation, `tsc --noEmit` for type-checking, ESM. References: `spec/ndsq.md`, the Rust crate `rust/ndsq/`, the Python package `python/ndsq/`, the frozen corpus `conformance/`.

---

## Scope of this plan

- **In:** the full TypeScript package mirroring the other two — values/bounds, domain canonicalization, three output-map kinds, transform canonicalization, all four desugarers, snake_case message/transform types, camelCase builders, `parse`/`normalize`, error codes, input validation, and the conformance runner.
- **Validation built in from the start.** Unlike the Python port (which added it later), this plan bakes the `require*` validators into the desugarers so the 5 structural-malformed → `invalid_json` corpus fixtures pass immediately. The corpus is the frozen contract — never edit a fixture to match a bug.
- **Node-native TS, erasable syntax only.** Node's type-stripping does NOT transform code, so: **no `enum`** (use `const` objects), no `namespace`, no constructor parameter properties, no decorators. Relative imports MUST include the `.ts` extension (`import { x } from "./errors.ts"`).
- **Deferred:** a publishable JS build (the package runs from `.ts` source under Node 24; a `dist` build via `tsc` is a noted follow-up). Negative `step`, deep `index_array` validation, and the i64/JS-number safe-integer limit follow the contract's deferrals.

## File structure

```
typescript/package.json            type:module, scripts (test, typecheck), dev deps (ajv, typescript, @types/node)
typescript/tsconfig.json           nodenext, strict, allowImportingTsExtensions, noEmit
typescript/src/errors.ts           Reason (const object) + ReasonCode + NdsqError
typescript/src/values.ts           IndexValue/BoundJson types, parseIndexValue, ParsedBound + parseBound/boundToJSON, require* validators
typescript/src/messages.ts         snake_case message union types + Transform/OutputMap JSON types
typescript/src/domain.ts           canonicalizeDomain -> domain JSON fields
typescript/src/output.ts           OutputMapJson union + canonicalizeOutputMap
typescript/src/transform.ts        Transform type + canonicalizeTransform + identityOutput
typescript/src/shorthand.ts        desugarPoint/Box/Slice/Points
typescript/src/builders.ts         camelCase builders: point/box/slice/points
typescript/src/index.ts            public API: parse, normalize, re-exports
typescript/test/*.test.ts          unit tests (node:test)
typescript/test/conformance.test.ts  corpus runner (ajv schema-validate + normalize + deepStrictEqual)
```

**Environment note for all tasks:** run every command from `/Users/d-v-b/dev/ndsq/typescript`. The corpus and schema live at the repo root (`/Users/d-v-b/dev/ndsq/conformance`, `/Users/d-v-b/dev/ndsq/schema`), reachable from a test file as `join(import.meta.dirname, "..", "..")`.

---

## Task 1: Scaffold the package

**Files:**
- Create: `typescript/package.json`, `typescript/tsconfig.json`, `typescript/src/index.ts` (stub), `typescript/.gitignore`

- [ ] **Step 1: Write `package.json`**

Create `typescript/package.json`:

```json
{
  "name": "ndsq",
  "version": "0.1.0",
  "description": "JSON-serializable n-dimensional spatial queries (TypeScript implementation)",
  "type": "module",
  "license": "MIT",
  "engines": { "node": ">=23.6" },
  "exports": { ".": "./src/index.ts" },
  "scripts": {
    "test": "node --test test/*.test.ts",
    "typecheck": "tsc --noEmit"
  },
  "devDependencies": {
    "@types/node": "^22",
    "ajv": "^8.17",
    "typescript": "^5.7"
  }
}
```

- [ ] **Step 2: Write `tsconfig.json`**

Create `typescript/tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "es2022",
    "module": "nodenext",
    "moduleResolution": "nodenext",
    "strict": true,
    "verbatimModuleSyntax": true,
    "allowImportingTsExtensions": true,
    "noEmit": true,
    "types": ["node"],
    "skipLibCheck": true
  },
  "include": ["src/**/*.ts", "test/**/*.ts"]
}
```

- [ ] **Step 3: Create the index stub and gitignore**

Create `typescript/src/index.ts`:

```typescript
// ndsq — JSON-serializable n-dimensional spatial queries (public API filled in Task 8).
export {};
```

Create `typescript/.gitignore`:

```
node_modules/
dist/
```

- [ ] **Step 4: Install and verify**

Run: `npm install` — Expected: installs `ajv`, `typescript`, `@types/node`; creates `package-lock.json` and `node_modules/`.

Run: `npx tsc --noEmit` — Expected: no output, exit 0 (the stub typechecks).

Run: `node --test test/*.test.ts 2>&1 | tail -3` — Expected: it runs (reports no test files / 0 tests; the `test/` dir doesn't exist yet, so "Could not find test files" is acceptable — the point is the runner executes). This is fine; Task 2 adds the first test.

- [ ] **Step 5: Commit**

```bash
git add typescript/package.json typescript/tsconfig.json typescript/src typescript/.gitignore typescript/package-lock.json
git commit -m "chore(ts): scaffold ndsq TypeScript package (Node-native TS, node:test, ajv)"
```

---

## Task 2: `errors.ts` — Reason codes and NdsqError

**Files:**
- Create: `typescript/src/errors.ts`
- Test: `typescript/test/errors.test.ts`

Use a `const` object for `Reason` (NOT a TS `enum` — enums emit runtime code that Node's type-stripping rejects).

- [ ] **Step 1: Write the failing test**

Create `typescript/test/errors.test.ts`:

```typescript
import { test } from "node:test";
import assert from "node:assert/strict";
import { NdsqError, Reason } from "../src/errors.ts";

test("reason codes are stable", () => {
  assert.equal(Reason.StepZero, "step_zero");
  assert.equal(Reason.NegativeStepUnsupported, "negative_step_unsupported");
  assert.equal(Reason.MultipleUpperBounds, "multiple_upper_bounds");
  assert.equal(Reason.RankMismatch, "rank_mismatch");
  assert.equal(Reason.UnknownKind, "unknown_kind");
  assert.equal(Reason.InvalidJson, "invalid_json");
});

test("error carries reason and detail", () => {
  const err = new NdsqError(Reason.StepZero, "step must be non-zero");
  assert.equal(err.reason, "step_zero");
  assert.ok(err instanceof NdsqError);
  assert.ok(err instanceof Error);
  assert.match(err.message, /step must be non-zero/);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test test/errors.test.ts 2>&1 | tail -5`
Expected: FAIL — cannot find module `../src/errors.ts`.

- [ ] **Step 3: Write the implementation**

Create `typescript/src/errors.ts`:

```typescript
/** Stable machine-readable reason codes for rejected messages (the wire codes). */
export const Reason = {
  StepZero: "step_zero",
  NegativeStepUnsupported: "negative_step_unsupported",
  MultipleUpperBounds: "multiple_upper_bounds",
  RankMismatch: "rank_mismatch",
  UnknownKind: "unknown_kind",
  InvalidJson: "invalid_json",
} as const;

export type ReasonCode = (typeof Reason)[keyof typeof Reason];

/** Raised when a message cannot be parsed or normalized. */
export class NdsqError extends Error {
  reason: ReasonCode;
  detail: string;

  constructor(reason: ReasonCode, detail: string) {
    super(`${reason}: ${detail}`);
    this.name = "NdsqError";
    this.reason = reason;
    this.detail = detail;
  }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test test/errors.test.ts 2>&1 | tail -5`
Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add typescript/src/errors.ts typescript/test/errors.test.ts
git commit -m "feat(ts): Reason codes and NdsqError"
```

---

## Task 3: `values.ts` — index values, bounds, and validators

**Files:**
- Create: `typescript/src/values.ts`
- Test: `typescript/test/values.test.ts`

`IndexValue` is `number | "-inf" | "+inf"`. `BoundJson` adds the `[n]`-bracket implicit form. `ParsedBound` is the internal `{value, implicit}` form. `require*` validators reject malformed scalars/arrays with `invalid_json`. **JS note:** `typeof true === "boolean"`, so `Number.isInteger` naturally rejects booleans (no Python-style bool-as-int trap), but still rejects floats/NaN.

- [ ] **Step 1: Write the failing test**

Create `typescript/test/values.test.ts`:

```typescript
import { test } from "node:test";
import assert from "node:assert/strict";
import { NdsqError } from "../src/errors.ts";
import { boundToJSON, parseBound, parseIndexValue, requireInt, requireIntArray, requireStringArray } from "../src/values.ts";

test("parseIndexValue accepts ints and sentinels", () => {
  assert.equal(parseIndexValue(7), 7);
  assert.equal(parseIndexValue("-inf"), "-inf");
  assert.equal(parseIndexValue("+inf"), "+inf");
});

test("parseIndexValue rejects bool, float, and garbage", () => {
  assert.throws(() => parseIndexValue(true), NdsqError);
  assert.throws(() => parseIndexValue(1.5), NdsqError);
  assert.throws(() => parseIndexValue("nope"), NdsqError);
});

test("bare value is explicit, bracket is implicit", () => {
  assert.deepEqual(parseBound(7), { value: 7, implicit: false });
  assert.deepEqual(parseBound([7]), { value: 7, implicit: true });
  assert.deepEqual(parseBound(["-inf"]), { value: "-inf", implicit: true });
});

test("multi-element bracket is invalid", () => {
  assert.throws(() => parseBound([1, 2]), NdsqError);
});

test("boundToJSON round-trips explicit and implicit", () => {
  assert.equal(boundToJSON({ value: 7, implicit: false }), 7);
  assert.deepEqual(boundToJSON({ value: 7, implicit: true }), [7]);
  assert.deepEqual(boundToJSON({ value: "-inf", implicit: true }), ["-inf"]);
});

test("require validators", () => {
  assert.equal(requireInt(3, "x"), 3);
  assert.throws(() => requireInt(true, "x"), NdsqError);
  assert.deepEqual(requireIntArray([1, 2], "x"), [1, 2]);
  assert.throws(() => requireIntArray(5, "x"), NdsqError);
  assert.throws(() => requireIntArray([true], "x"), NdsqError);
  assert.deepEqual(requireStringArray(["a", ""], "x"), ["a", ""]);
  assert.throws(() => requireStringArray([1], "x"), NdsqError);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test test/values.test.ts 2>&1 | tail -5`
Expected: FAIL — cannot find `../src/values.ts`.

- [ ] **Step 3: Write the implementation**

Create `typescript/src/values.ts`:

```typescript
import { NdsqError, Reason } from "./errors.ts";

/** A single index coordinate: a finite integer, or an infinity sentinel. */
export type IndexValue = number | "-inf" | "+inf";

/** A JSON-level bound: a value, or that value wrapped in a 1-element array (implicit). */
export type BoundJson = IndexValue | [IndexValue];

/** Internal parsed bound: the value plus its explicit/implicit flag. */
export interface ParsedBound {
  value: IndexValue;
  implicit: boolean;
}

export function parseIndexValue(raw: unknown): IndexValue {
  if (typeof raw === "number" && Number.isInteger(raw)) return raw;
  if (raw === "-inf" || raw === "+inf") return raw;
  throw new NdsqError(Reason.InvalidJson, `invalid index value: ${JSON.stringify(raw)}`);
}

export function parseBound(raw: unknown): ParsedBound {
  if (Array.isArray(raw)) {
    if (raw.length !== 1) {
      throw new NdsqError(Reason.InvalidJson, "implicit bound must be a 1-element array");
    }
    return { value: parseIndexValue(raw[0]), implicit: true };
  }
  return { value: parseIndexValue(raw), implicit: false };
}

export function boundToJSON(b: ParsedBound): BoundJson {
  return b.implicit ? [b.value] : b.value;
}

export function requireInt(raw: unknown, what: string): number {
  if (typeof raw === "number" && Number.isInteger(raw)) return raw;
  throw new NdsqError(Reason.InvalidJson, `${what} must be an integer, got ${JSON.stringify(raw)}`);
}

export function requireArray(raw: unknown, what: string): unknown[] {
  if (Array.isArray(raw)) return raw;
  throw new NdsqError(Reason.InvalidJson, `${what} must be an array, got ${JSON.stringify(raw)}`);
}

export function requireIntArray(raw: unknown, what: string): number[] {
  return requireArray(raw, what).map((v, i) => requireInt(v, `${what}[${i}]`));
}

export function requireStringArray(raw: unknown, what: string): string[] {
  return requireArray(raw, what).map((v, i) => {
    if (typeof v !== "string") {
      throw new NdsqError(Reason.InvalidJson, `${what}[${i}] must be a string, got ${JSON.stringify(v)}`);
    }
    return v;
  });
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test test/values.test.ts 2>&1 | tail -5`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add typescript/src/values.ts typescript/test/values.test.ts
git commit -m "feat(ts): index values, bounds, and require validators"
```

---

## Task 4: `messages.ts` — snake_case wire types

**Files:**
- Create: `typescript/src/messages.ts`
- Test: `typescript/test/messages.test.ts` (compile-only type assertions)

These mirror the JSON exactly (snake_case keys). No runtime code beyond the type-level checks in the test.

- [ ] **Step 1: Write the failing test**

Create `typescript/test/messages.test.ts`:

```typescript
import { test } from "node:test";
import assert from "node:assert/strict";
import type { BoxMessage, Message, SliceMessage } from "../src/messages.ts";

test("message literals are typed against the union", () => {
  // Compile-time checks; the assertions just ensure the file is exercised at runtime.
  const box: BoxMessage = { kind: "box", inclusive_min: [0, 0], exclusive_max: [3, 4] };
  const slice: SliceMessage = { kind: "slice", start: [0], stop: [10], step: [2] };
  const messages: Message[] = [box, slice, { kind: "point", coords: [1] }];
  assert.equal(box.kind, "box");
  assert.equal(slice.step?.[0], 2);
  assert.equal(messages.length, 3);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test test/messages.test.ts 2>&1 | tail -5`
Expected: FAIL — cannot find `../src/messages.ts`.

- [ ] **Step 3: Write the implementation**

Create `typescript/src/messages.ts`:

```typescript
import type { BoundJson, IndexValue } from "./values.ts";

export interface PointMessage {
  kind: "point";
  coords: number[];
}

export interface BoxMessage {
  kind: "box";
  inclusive_min?: BoundJson[];
  exclusive_max?: BoundJson[];
  inclusive_max?: BoundJson[];
  shape?: BoundJson[];
  labels?: string[];
}

export interface SliceMessage {
  kind: "slice";
  start: number[];
  stop: number[];
  step?: number[];
  labels?: string[];
}

export interface PointsMessage {
  kind: "points";
  coords: number[][];
}

/** An output-map object as accepted on input (before default-filling). */
export interface OutputMapInput {
  offset?: number;
  stride?: number;
  input_dimension?: number;
  index_array?: unknown;
  index_array_bounds?: [IndexValue, IndexValue];
}

export interface TransformMessage {
  kind: "transform";
  input_rank?: number;
  input_inclusive_min?: BoundJson[];
  input_exclusive_max?: BoundJson[];
  input_inclusive_max?: BoundJson[];
  input_shape?: BoundJson[];
  input_labels?: string[];
  output?: OutputMapInput[];
}

/** Any ndsq message, discriminated on `kind`. */
export type Message = PointMessage | BoxMessage | SliceMessage | PointsMessage | TransformMessage;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test test/messages.test.ts 2>&1 | tail -5`
Expected: 1 test passes.

- [ ] **Step 5: Commit**

```bash
git add typescript/src/messages.ts typescript/test/messages.test.ts
git commit -m "feat(ts): snake_case wire types for messages"
```

---

## Task 5: `domain.ts` — canonicalizeDomain

**Files:**
- Create: `typescript/src/domain.ts`
- Test: `typescript/test/domain.test.ts`

`canonicalizeDomain` takes the raw JSON-level fields and returns the canonical domain portion of a transform body (snake_case JSON), mirroring the Rust/Python canonicalization.

- [ ] **Step 1: Write the failing test**

Create `typescript/test/domain.test.ts`:

```typescript
import { test } from "node:test";
import assert from "node:assert/strict";
import { NdsqError } from "../src/errors.ts";
import { canonicalizeDomain } from "../src/domain.ts";

test("shape only defaults min to zero", () => {
  const d = canonicalizeDomain({ shape: [3, 4] });
  assert.deepStrictEqual(d, {
    input_rank: 2,
    input_inclusive_min: [0, 0],
    input_exclusive_max: [3, 4],
    input_labels: ["", ""],
  });
});

test("inclusive_max converts to exclusive", () => {
  const d = canonicalizeDomain({ inclusive_min: [0], inclusive_max: [9] });
  assert.deepStrictEqual(d.input_exclusive_max, [10]);
});

test("implicit and infinite bounds preserved", () => {
  const d = canonicalizeDomain({ inclusive_min: [["-inf"], 0], exclusive_max: [["+inf"], 4] });
  assert.deepStrictEqual(d.input_inclusive_min, [["-inf"], 0]);
  assert.deepStrictEqual(d.input_exclusive_max, [["+inf"], 4]);
});

test("multiple upper bounds is error", () => {
  assert.throws(() => canonicalizeDomain({ shape: [3], exclusive_max: [3] }), (e) => e instanceof NdsqError && e.reason === "multiple_upper_bounds");
});

test("length disagreement is rank_mismatch", () => {
  assert.throws(() => canonicalizeDomain({ inclusive_min: [0, 0], shape: [3] }), (e) => e instanceof NdsqError && e.reason === "rank_mismatch");
});

test("non-list bound is invalid_json", () => {
  assert.throws(() => canonicalizeDomain({ inclusive_min: 5 }), (e) => e instanceof NdsqError && e.reason === "invalid_json");
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test test/domain.test.ts 2>&1 | tail -5`
Expected: FAIL — cannot find `../src/domain.ts`.

- [ ] **Step 3: Write the implementation**

Create `typescript/src/domain.ts`:

```typescript
import { NdsqError, Reason } from "./errors.ts";
import {
  type BoundJson,
  type IndexValue,
  type ParsedBound,
  boundToJSON,
  parseBound,
  requireArray,
  requireInt,
  requireStringArray,
} from "./values.ts";

export interface DomainFields {
  input_rank: number;
  input_inclusive_min: BoundJson[];
  input_exclusive_max: BoundJson[];
  input_labels: string[];
}

function bumpInclusiveToExclusive(b: ParsedBound): ParsedBound {
  const value: IndexValue = typeof b.value === "number" ? b.value + 1 : b.value;
  return { value, implicit: b.implicit };
}

function addShape(lo: ParsedBound, sz: ParsedBound): ParsedBound {
  let value: IndexValue;
  if (typeof lo.value === "number" && typeof sz.value === "number") value = lo.value + sz.value;
  else if (lo.value === "+inf" || sz.value === "+inf") value = "+inf";
  else value = "-inf";
  return { value, implicit: lo.implicit };
}

export interface RawDomainFields {
  rank?: unknown;
  inclusive_min?: unknown;
  exclusive_max?: unknown;
  inclusive_max?: unknown;
  shape?: unknown;
  labels?: unknown;
}

export function canonicalizeDomain(fields: RawDomainFields): DomainFields {
  const parseBounds = (raw: unknown, name: string): ParsedBound[] | null =>
    raw === undefined ? null : requireArray(raw, name).map((b) => parseBound(b));

  const imin = parseBounds(fields.inclusive_min, "inclusive_min");
  const emax = parseBounds(fields.exclusive_max, "exclusive_max");
  const incmax = parseBounds(fields.inclusive_max, "inclusive_max");
  const shp = parseBounds(fields.shape, "shape");
  const labels = fields.labels === undefined ? null : requireStringArray(fields.labels, "labels");
  let rank = fields.rank === undefined ? null : requireInt(fields.rank, "rank");
  if (rank !== null && rank < 0) {
    throw new NdsqError(Reason.InvalidJson, `rank must be non-negative, got ${rank}`);
  }

  const upperCount = (emax !== null ? 1 : 0) + (incmax !== null ? 1 : 0) + (shp !== null ? 1 : 0);
  if (upperCount > 1) {
    throw new NdsqError(Reason.MultipleUpperBounds, "specify only one of exclusive_max / inclusive_max / shape");
  }

  let resolved = rank;
  for (const arr of [imin, emax, incmax, shp, labels]) {
    if (arr === null) continue;
    if (resolved !== null && resolved !== arr.length) {
      throw new NdsqError(Reason.RankMismatch, `array length ${arr.length} disagrees with rank ${resolved}`);
    }
    resolved = arr.length;
  }
  const r = resolved ?? 0;

  const iminFinal: ParsedBound[] =
    imin ?? Array.from({ length: r }, () => ({ value: 0, implicit: false }) as ParsedBound);

  let exclusive: ParsedBound[];
  if (emax !== null) exclusive = emax;
  else if (incmax !== null) exclusive = incmax.map(bumpInclusiveToExclusive);
  else if (shp !== null) exclusive = iminFinal.map((lo, i) => addShape(lo, shp[i]));
  else exclusive = Array.from({ length: r }, () => ({ value: "+inf", implicit: true }) as ParsedBound);

  return {
    input_rank: r,
    input_inclusive_min: iminFinal.map(boundToJSON),
    input_exclusive_max: exclusive.map(boundToJSON),
    input_labels: labels ?? Array.from({ length: r }, () => ""),
  };
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test test/domain.test.ts 2>&1 | tail -5`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add typescript/src/domain.ts typescript/test/domain.test.ts
git commit -m "feat(ts): canonicalizeDomain (upper-bound canonicalization)"
```

---

## Task 6: `output.ts` — output-map canonicalization

**Files:**
- Create: `typescript/src/output.ts`
- Test: `typescript/test/output.test.ts`

The canonical output map IS its JSON shape (plain object). Discrimination: `index_array` → index-array map; else `input_dimension` → single-input-dimension; else constant.

- [ ] **Step 1: Write the failing test**

Create `typescript/test/output.test.ts`:

```typescript
import { test } from "node:test";
import assert from "node:assert/strict";
import { canonicalizeOutputMap } from "../src/output.ts";

test("constant map carries only offset", () => {
  assert.deepStrictEqual(canonicalizeOutputMap({ offset: 3 }), { offset: 3 });
});

test("single_input_dimension fills defaults", () => {
  assert.deepStrictEqual(canonicalizeOutputMap({ input_dimension: 2 }), {
    offset: 0,
    stride: 1,
    input_dimension: 2,
  });
});

test("index_array fills offset, stride, bounds", () => {
  assert.deepStrictEqual(canonicalizeOutputMap({ index_array: [1, 2, 3] }), {
    offset: 0,
    stride: 1,
    index_array: [1, 2, 3],
    index_array_bounds: ["-inf", "+inf"],
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test test/output.test.ts 2>&1 | tail -5`
Expected: FAIL — cannot find `../src/output.ts`.

- [ ] **Step 3: Write the implementation**

Create `typescript/src/output.ts`:

```typescript
import { NdsqError, Reason } from "./errors.ts";
import { type IndexValue, parseIndexValue, requireInt } from "./values.ts";

/** A canonical output map (its JSON shape). */
export type OutputMapJson =
  | { offset: number }
  | { offset: number; stride: number; input_dimension: number }
  | { offset: number; stride: number; index_array: unknown; index_array_bounds: [IndexValue, IndexValue] };

export function canonicalizeOutputMap(raw: unknown): OutputMapJson {
  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) {
    throw new NdsqError(Reason.InvalidJson, `output map must be an object, got ${JSON.stringify(raw)}`);
  }
  const m = raw as Record<string, unknown>;
  const offset = "offset" in m ? requireInt(m.offset, "output.offset") : 0;
  const stride = "stride" in m ? requireInt(m.stride, "output.stride") : 1;

  if ("index_array" in m) {
    const b = "index_array_bounds" in m ? m.index_array_bounds : ["-inf", "+inf"];
    if (!Array.isArray(b) || b.length !== 2) {
      throw new NdsqError(Reason.InvalidJson, "index_array_bounds must be a 2-element array");
    }
    return {
      offset,
      stride,
      index_array: m.index_array,
      index_array_bounds: [parseIndexValue(b[0]), parseIndexValue(b[1])],
    };
  }
  if ("input_dimension" in m) {
    const dim = requireInt(m.input_dimension, "output.input_dimension");
    if (dim < 0) {
      throw new NdsqError(Reason.InvalidJson, `input_dimension must be non-negative, got ${dim}`);
    }
    return { offset, stride, input_dimension: dim };
  }
  return { offset };
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test test/output.test.ts 2>&1 | tail -5`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add typescript/src/output.ts typescript/test/output.test.ts
git commit -m "feat(ts): output-map canonicalization"
```

---

## Task 7: `transform.ts` — Transform and canonicalizeTransform

**Files:**
- Create: `typescript/src/transform.ts`
- Test: `typescript/test/transform.test.ts`

- [ ] **Step 1: Write the failing test**

Create `typescript/test/transform.test.ts`:

```typescript
import { test } from "node:test";
import assert from "node:assert/strict";
import { canonicalizeTransform } from "../src/transform.ts";
import type { TransformMessage } from "../src/messages.ts";

test("omitted output becomes explicit identity", () => {
  const t = canonicalizeTransform({ kind: "transform", input_inclusive_min: [0, 0], input_exclusive_max: [3, 4] });
  assert.deepStrictEqual(t.output, [
    { offset: 0, stride: 1, input_dimension: 0 },
    { offset: 0, stride: 1, input_dimension: 1 },
  ]);
  assert.deepStrictEqual(t.input_inclusive_min, [0, 0]);
});

test("canonicalize is idempotent", () => {
  const once = canonicalizeTransform({ kind: "transform", input_inclusive_min: [0], input_shape: [5] });
  const twice = canonicalizeTransform({ kind: "transform", ...once } as TransformMessage);
  assert.deepStrictEqual(once, twice);
});

test("explicit output all three kinds", () => {
  const t = canonicalizeTransform({
    kind: "transform",
    input_inclusive_min: [0],
    input_exclusive_max: [3],
    output: [{ offset: 7 }, { input_dimension: 0, stride: 2 }, { index_array: [1, 2, 3] }],
  });
  assert.deepStrictEqual(t.output, [
    { offset: 7 },
    { offset: 0, stride: 2, input_dimension: 0 },
    { offset: 0, stride: 1, index_array: [1, 2, 3], index_array_bounds: ["-inf", "+inf"] },
  ]);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test test/transform.test.ts 2>&1 | tail -5`
Expected: FAIL — cannot find `../src/transform.ts`.

- [ ] **Step 3: Write the implementation**

Create `typescript/src/transform.ts`:

```typescript
import { type DomainFields, canonicalizeDomain } from "./domain.ts";
import type { TransformMessage } from "./messages.ts";
import { type OutputMapJson, canonicalizeOutputMap } from "./output.ts";
import { requireArray } from "./values.ts";

/** The canonical core: the domain fields plus explicit output maps (a JSON body). */
export interface Transform extends DomainFields {
  output: OutputMapJson[];
}

export function identityOutput(rank: number): OutputMapJson[] {
  return Array.from({ length: rank }, (_unused, k) => ({ offset: 0, stride: 1, input_dimension: k }));
}

export function canonicalizeTransform(msg: TransformMessage): Transform {
  const domain = canonicalizeDomain({
    rank: msg.input_rank,
    inclusive_min: msg.input_inclusive_min,
    exclusive_max: msg.input_exclusive_max,
    inclusive_max: msg.input_inclusive_max,
    shape: msg.input_shape,
    labels: msg.input_labels,
  });
  const output =
    msg.output === undefined
      ? identityOutput(domain.input_rank)
      : requireArray(msg.output, "output").map(canonicalizeOutputMap);
  return { ...domain, output };
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test test/transform.test.ts 2>&1 | tail -5`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add typescript/src/transform.ts typescript/test/transform.test.ts
git commit -m "feat(ts): Transform and canonicalizeTransform (explicit identity output)"
```

---

## Task 8: `shorthand.ts` (point, box) + `builders.ts` + `index.ts` (parse, normalize)

**Files:**
- Create: `typescript/src/shorthand.ts`, `typescript/src/builders.ts`
- Modify: `typescript/src/index.ts`
- Test: `typescript/test/point_box.test.ts`

- [ ] **Step 1: Write the failing test**

Create `typescript/test/point_box.test.ts`:

```typescript
import { test } from "node:test";
import assert from "node:assert/strict";
import { box, normalize, parse } from "../src/index.ts";

function norm(text: string): unknown {
  return normalize(parse(text));
}

test("point desugars to constant maps", () => {
  const d = norm('{"kind": "point", "coords": [4, 7]}') as Record<string, unknown>;
  assert.equal(d.input_rank, 0);
  assert.deepStrictEqual(d.input_inclusive_min, []);
  assert.deepStrictEqual(d.output, [{ offset: 4 }, { offset: 7 }]);
});

test("box desugars to identity", () => {
  const d = norm('{"kind": "box", "inclusive_min": [0, 0], "exclusive_max": [3, 4]}') as Record<string, unknown>;
  assert.deepStrictEqual(d.output, [
    { offset: 0, stride: 1, input_dimension: 0 },
    { offset: 0, stride: 1, input_dimension: 1 },
  ]);
});

test("box shape-only defaults origin zero", () => {
  const d = norm('{"kind": "box", "shape": [5]}') as Record<string, unknown>;
  assert.deepStrictEqual(d.input_inclusive_min, [0]);
  assert.deepStrictEqual(d.input_exclusive_max, [5]);
});

test("normalize works with a camelCase builder", () => {
  const d = normalize(box({ inclusiveMin: [0, 0], exclusiveMax: [3, 4] })) as Record<string, unknown>;
  assert.deepStrictEqual(d.input_exclusive_max, [3, 4]);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test test/point_box.test.ts 2>&1 | tail -5`
Expected: FAIL — cannot find `box`/`normalize`/`parse` from `../src/index.ts`.

- [ ] **Step 3: Write `shorthand.ts` (point + box; slice/points throw for now)**

Create `typescript/src/shorthand.ts`:

```typescript
import { canonicalizeDomain } from "./domain.ts";
import type { BoxMessage, PointMessage, PointsMessage, SliceMessage } from "./messages.ts";
import type { OutputMapJson } from "./output.ts";
import { type Transform, identityOutput } from "./transform.ts";
import { requireIntArray } from "./values.ts";

export function desugarPoint(msg: PointMessage): Transform {
  const coords = requireIntArray((msg as { coords?: unknown }).coords, "point.coords");
  return {
    input_rank: 0,
    input_inclusive_min: [],
    input_exclusive_max: [],
    input_labels: [],
    output: coords.map((c) => ({ offset: c })),
  };
}

export function desugarBox(msg: BoxMessage): Transform {
  const domain = canonicalizeDomain({
    inclusive_min: msg.inclusive_min,
    exclusive_max: msg.exclusive_max,
    inclusive_max: msg.inclusive_max,
    shape: msg.shape,
    labels: msg.labels,
  });
  return { ...domain, output: identityOutput(domain.input_rank) };
}

export function desugarSlice(_msg: SliceMessage): Transform {
  throw new Error("desugarSlice — implemented in Task 9");
}

export function desugarPoints(_msg: PointsMessage): Transform {
  throw new Error("desugarPoints — implemented in Task 10");
}

export type { OutputMapJson };
```

- [ ] **Step 4: Write `builders.ts`**

Create `typescript/src/builders.ts`:

```typescript
import type { BoundJson } from "./values.ts";
import type { BoxMessage, PointMessage, PointsMessage, SliceMessage } from "./messages.ts";

export function point(coords: number[]): PointMessage {
  return { kind: "point", coords };
}

export interface BoxOptions {
  inclusiveMin?: BoundJson[];
  exclusiveMax?: BoundJson[];
  inclusiveMax?: BoundJson[];
  shape?: BoundJson[];
  labels?: string[];
}

export function box(opts: BoxOptions = {}): BoxMessage {
  const msg: BoxMessage = { kind: "box" };
  if (opts.inclusiveMin !== undefined) msg.inclusive_min = opts.inclusiveMin;
  if (opts.exclusiveMax !== undefined) msg.exclusive_max = opts.exclusiveMax;
  if (opts.inclusiveMax !== undefined) msg.inclusive_max = opts.inclusiveMax;
  if (opts.shape !== undefined) msg.shape = opts.shape;
  if (opts.labels !== undefined) msg.labels = opts.labels;
  return msg;
}

export interface SliceOptions {
  start: number[];
  stop: number[];
  step?: number[];
  labels?: string[];
}

export function slice(opts: SliceOptions): SliceMessage {
  const msg: SliceMessage = { kind: "slice", start: opts.start, stop: opts.stop };
  if (opts.step !== undefined) msg.step = opts.step;
  if (opts.labels !== undefined) msg.labels = opts.labels;
  return msg;
}

export function points(coords: number[][]): PointsMessage {
  return { kind: "points", coords };
}
```

- [ ] **Step 5: Write the public API in `index.ts`**

Replace `typescript/src/index.ts`:

```typescript
/**
 * ndsq — JSON-serializable n-dimensional spatial queries.
 *
 * Parse a JSON string with `parse`, then reduce it to a canonical `Transform`
 * with `normalize`. Construct messages ergonomically with the camelCase
 * builders (`point`/`box`/`slice`/`points`).
 */

import { NdsqError, Reason } from "./errors.ts";
import type { Message } from "./messages.ts";
import { desugarBox, desugarPoint, desugarPoints, desugarSlice } from "./shorthand.ts";
import { type Transform, canonicalizeTransform } from "./transform.ts";

export { NdsqError, Reason };
export type { ReasonCode } from "./errors.ts";
export type {
  BoxMessage,
  Message,
  OutputMapInput,
  PointMessage,
  PointsMessage,
  SliceMessage,
  TransformMessage,
} from "./messages.ts";
export type { Transform } from "./transform.ts";
export type { OutputMapJson } from "./output.ts";
export type { DomainFields } from "./domain.ts";
export type { BoundJson, IndexValue } from "./values.ts";
export { type BoxOptions, type SliceOptions, box, point, points, slice } from "./builders.ts";

const KNOWN_KINDS = new Set(["point", "box", "slice", "points", "transform"]);

/** Parse a JSON string and validate the `kind` discriminator. */
export function parse(text: string): Message {
  let obj: unknown;
  try {
    obj = JSON.parse(text);
  } catch (err) {
    throw new NdsqError(Reason.InvalidJson, err instanceof Error ? err.message : String(err));
  }
  if (typeof obj !== "object" || obj === null || Array.isArray(obj)) {
    throw new NdsqError(Reason.InvalidJson, "message must be a JSON object");
  }
  const kind = (obj as { kind?: unknown }).kind;
  if (typeof kind !== "string") {
    throw new NdsqError(Reason.InvalidJson, "missing string `kind`");
  }
  if (!KNOWN_KINDS.has(kind)) {
    throw new NdsqError(Reason.UnknownKind, `unknown kind: ${kind}`);
  }
  return obj as Message;
}

/** Reduce a message to its canonical Transform. */
export function normalize(message: Message): Transform {
  switch (message.kind) {
    case "point":
      return desugarPoint(message);
    case "box":
      return desugarBox(message);
    case "slice":
      return desugarSlice(message);
    case "points":
      return desugarPoints(message);
    case "transform":
      return canonicalizeTransform(message);
    default: {
      const k = (message as { kind?: unknown }).kind;
      throw new NdsqError(Reason.UnknownKind, `unknown kind: ${String(k)}`);
    }
  }
}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `node --test test/point_box.test.ts 2>&1 | tail -5`
Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add typescript/src/shorthand.ts typescript/src/builders.ts typescript/src/index.ts typescript/test/point_box.test.ts
git commit -m "feat(ts): desugar point and box; camelCase builders; parse/normalize API"
```

---

## Task 9: `shorthand.ts` — slice desugaring

**Files:**
- Modify: `typescript/src/shorthand.ts`
- Test: `typescript/test/slice.test.ts`

Implements spec §5.3 for `s > 0`: `m = max(0, ceil((b-a)/s))`, `o = Math.floor(a/s)` (floor for positive `s`, matching Rust `div_euclid` / Python `//`), `offset = a - s*o`, domain `[o, o+m)`. `s == 0` → `step_zero`; `s < 0` → `negative_step_unsupported`. Coordinate-preserving (NOT re-based to `[0, m)`).

- [ ] **Step 1: Write the failing test**

Create `typescript/test/slice.test.ts`:

```typescript
import { test } from "node:test";
import assert from "node:assert/strict";
import { NdsqError } from "../src/errors.ts";
import { normalize, parse } from "../src/index.ts";

function norm(text: string): Record<string, unknown> {
  return normalize(parse(text)) as unknown as Record<string, unknown>;
}

function reason(text: string): string {
  try {
    normalize(parse(text));
  } catch (e) {
    if (e instanceof NdsqError) return e.reason;
  }
  throw new Error("expected NdsqError");
}

test("unit step preserves source frame", () => {
  const d = norm('{"kind": "slice", "start": [5], "stop": [10]}');
  assert.deepStrictEqual(d.input_inclusive_min, [5]);
  assert.deepStrictEqual(d.input_exclusive_max, [10]);
  assert.deepStrictEqual(d.output, [{ offset: 0, stride: 1, input_dimension: 0 }]);
});

test("divisible strided slice", () => {
  const d = norm('{"kind": "slice", "start": [4], "stop": [10], "step": [2]}');
  assert.deepStrictEqual(d.input_inclusive_min, [2]);
  assert.deepStrictEqual(d.input_exclusive_max, [5]);
  assert.deepStrictEqual(d.output, [{ offset: 0, stride: 2, input_dimension: 0 }]);
});

test("nondivisible strided slice phase offset", () => {
  const d = norm('{"kind": "slice", "start": [5], "stop": [10], "step": [2]}');
  assert.deepStrictEqual(d.input_inclusive_min, [2]);
  assert.deepStrictEqual(d.output, [{ offset: 1, stride: 2, input_dimension: 0 }]);
});

test("negative start uses floor division", () => {
  // x[-3:3:2] selects -3,-1,1; o=floor(-3/2)=-2, offset=-3-2*(-2)=1, domain [-2,1)
  const d = norm('{"kind": "slice", "start": [-3], "stop": [3], "step": [2]}');
  assert.deepStrictEqual(d.input_inclusive_min, [-2]);
  assert.deepStrictEqual(d.input_exclusive_max, [1]);
  assert.deepStrictEqual(d.output, [{ offset: 1, stride: 2, input_dimension: 0 }]);
});

test("empty slice zero length", () => {
  const d = norm('{"kind": "slice", "start": [10], "stop": [10]}');
  assert.deepStrictEqual(d.input_inclusive_min, [10]);
  assert.deepStrictEqual(d.input_exclusive_max, [10]);
});

test("2d mixed step", () => {
  const d = norm('{"kind": "slice", "start": [0, 5], "stop": [10, 10], "step": [2, 1]}');
  assert.deepStrictEqual(d.input_inclusive_min, [0, 5]);
  assert.deepStrictEqual(d.input_exclusive_max, [5, 10]);
});

test("errors", () => {
  assert.equal(reason('{"kind": "slice", "start": [0], "stop": [4], "step": [0]}'), "step_zero");
  assert.equal(reason('{"kind": "slice", "start": [9], "stop": [0], "step": [-2]}'), "negative_step_unsupported");
  assert.equal(reason('{"kind": "slice", "start": [0, 0], "stop": [4]}'), "rank_mismatch");
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test test/slice.test.ts 2>&1 | tail -5`
Expected: FAIL — `desugarSlice — implemented in Task 9` thrown.

- [ ] **Step 3: Write the implementation**

In `typescript/src/shorthand.ts`, add the imports `NdsqError`, `Reason`, `BoundJson`, `requireIntArray`, `requireStringArray` (extend the existing import lines), and replace `desugarSlice` with:

```typescript
function ceilDiv(p: number, q: number): number {
  // Ceiling of p/q for p >= 0, q > 0.
  return Math.floor((p + q - 1) / q);
}

export function desugarSlice(msg: SliceMessage): Transform {
  const m = msg as { start?: unknown; stop?: unknown; step?: unknown; labels?: unknown };
  const start = requireIntArray(m.start, "slice.start");
  const stop = requireIntArray(m.stop, "slice.stop");
  const rank = start.length;
  if (stop.length !== rank) {
    throw new NdsqError(Reason.RankMismatch, "start and stop must have equal length");
  }
  let step: number[];
  if (m.step === undefined) {
    step = Array.from({ length: rank }, () => 1);
  } else {
    step = requireIntArray(m.step, "slice.step");
    if (step.length !== rank) {
      throw new NdsqError(Reason.RankMismatch, "step length must match start/stop");
    }
  }
  let labels: string[] | null = null;
  if (m.labels !== undefined) {
    labels = requireStringArray(m.labels, "slice.labels");
    if (labels.length !== rank) {
      throw new NdsqError(Reason.RankMismatch, "labels length must match start/stop");
    }
  }

  const inclusiveMin: BoundJson[] = [];
  const exclusiveMax: BoundJson[] = [];
  const output: OutputMapJson[] = [];
  for (let k = 0; k < rank; k++) {
    const a = start[k];
    const b = stop[k];
    const s = step[k];
    if (s === 0) throw new NdsqError(Reason.StepZero, "step must be non-zero");
    if (s < 0) throw new NdsqError(Reason.NegativeStepUnsupported, "negative step is not yet specified");
    const count = b <= a ? 0 : ceilDiv(b - a, s);
    const o = Math.floor(a / s); // floor(a/s) for s > 0
    const offset = a - s * o; // lattice phase in [0, s)
    inclusiveMin.push(o);
    exclusiveMax.push(o + count);
    output.push({ offset, stride: s, input_dimension: k });
  }

  return {
    input_rank: rank,
    input_inclusive_min: inclusiveMin,
    input_exclusive_max: exclusiveMax,
    input_labels: labels ?? Array.from({ length: rank }, () => ""),
    output,
  };
}
```

The updated import block at the top of `shorthand.ts` should read:

```typescript
import { NdsqError, Reason } from "./errors.ts";
import { canonicalizeDomain } from "./domain.ts";
import type { BoxMessage, PointMessage, PointsMessage, SliceMessage } from "./messages.ts";
import type { OutputMapJson } from "./output.ts";
import { type Transform, identityOutput } from "./transform.ts";
import { type BoundJson, requireIntArray, requireStringArray } from "./values.ts";
```

(Remove the now-redundant `export type { OutputMapJson };` line if present; keep using `OutputMapJson` as an imported type.)

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test test/slice.test.ts 2>&1 | tail -5`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add typescript/src/shorthand.ts typescript/test/slice.test.ts
git commit -m "feat(ts): desugar slice (positive step, coordinate-preserving)"
```

---

## Task 10: `shorthand.ts` — points desugaring

**Files:**
- Modify: `typescript/src/shorthand.ts`
- Test: `typescript/test/points.test.ts`

Implements spec §5.4: `m` points of rank `n` → `input_rank: 1`, domain `[0, m)`, `n` columnar index-array maps. Ragged points → `rank_mismatch`.

- [ ] **Step 1: Write the failing test**

Create `typescript/test/points.test.ts`:

```typescript
import { test } from "node:test";
import assert from "node:assert/strict";
import { NdsqError } from "../src/errors.ts";
import { normalize, parse } from "../src/index.ts";

function norm(text: string): Record<string, unknown> {
  return normalize(parse(text)) as unknown as Record<string, unknown>;
}

test("points transpose to columnar index arrays", () => {
  const d = norm('{"kind": "points", "coords": [[1, 10], [2, 20], [3, 30]]}');
  assert.equal(d.input_rank, 1);
  assert.deepStrictEqual(d.input_exclusive_max, [3]);
  assert.deepStrictEqual(d.output, [
    { offset: 0, stride: 1, index_array: [1, 2, 3], index_array_bounds: ["-inf", "+inf"] },
    { offset: 0, stride: 1, index_array: [10, 20, 30], index_array_bounds: ["-inf", "+inf"] },
  ]);
});

test("empty points", () => {
  const d = norm('{"kind": "points", "coords": []}');
  assert.equal(d.input_rank, 1);
  assert.deepStrictEqual(d.input_exclusive_max, [0]);
  assert.deepStrictEqual(d.output, []);
});

test("ragged points is rank_mismatch", () => {
  assert.throws(
    () => normalize(parse('{"kind": "points", "coords": [[1, 2], [3]]}')),
    (e) => e instanceof NdsqError && e.reason === "rank_mismatch",
  );
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test test/points.test.ts 2>&1 | tail -5`
Expected: FAIL — `desugarPoints — implemented in Task 10` thrown.

- [ ] **Step 3: Write the implementation**

In `typescript/src/shorthand.ts`, add `requireArray` to the `./values.ts` import, and replace `desugarPoints` with:

```typescript
export function desugarPoints(msg: PointsMessage): Transform {
  const rawCoords = requireArray((msg as { coords?: unknown }).coords, "points.coords");
  const coords = rawCoords.map((p, i) => requireIntArray(p, `points.coords[${i}]`));
  const m = coords.length;
  const n = coords.length > 0 ? coords[0].length : 0;
  for (const p of coords) {
    if (p.length !== n) {
      throw new NdsqError(Reason.RankMismatch, "all points must have equal dimensionality");
    }
  }

  const output: OutputMapJson[] = [];
  for (let k = 0; k < n; k++) {
    const column = coords.map((p) => p[k]);
    output.push({ offset: 0, stride: 1, index_array: column, index_array_bounds: ["-inf", "+inf"] });
  }
  return {
    input_rank: 1,
    input_inclusive_min: [0],
    input_exclusive_max: [m],
    input_labels: [""],
    output,
  };
}
```

The `./values.ts` import line becomes:

```typescript
import { type BoundJson, requireArray, requireIntArray, requireStringArray } from "./values.ts";
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test test/points.test.ts 2>&1 | tail -5`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add typescript/src/shorthand.ts typescript/test/points.test.ts
git commit -m "feat(ts): desugar points (row-major to columnar index arrays)"
```

---

## Task 11: parse / dispatch / malformed-input tests

**Files:**
- Test: `typescript/test/parse.test.ts`

`parse`/`normalize` and the desugarers' validators already implement this. These tests lock the behavior, including the structural-malformed → `invalid_json` cases that the shared corpus requires.

- [ ] **Step 1: Write the tests**

Create `typescript/test/parse.test.ts`:

```typescript
import { test } from "node:test";
import assert from "node:assert/strict";
import { NdsqError } from "../src/errors.ts";
import { normalize, parse } from "../src/index.ts";

function reason(text: string): string {
  try {
    normalize(parse(text));
  } catch (e) {
    if (e instanceof NdsqError) return e.reason;
  }
  throw new Error("expected NdsqError");
}

test("kind dispatch errors", () => {
  assert.equal(reason('{"kind": "bogus"}'), "unknown_kind");
  assert.equal(reason('{"coords": [1]}'), "invalid_json"); // missing kind
  assert.equal(reason("{ not json"), "invalid_json");
  assert.equal(reason("[1, 2, 3]"), "invalid_json"); // non-object
});

test("malformed bodies are invalid_json (Rust/Python parity)", () => {
  assert.equal(reason('{"kind": "point"}'), "invalid_json"); // missing coords
  assert.equal(reason('{"kind": "point", "coords": [true]}'), "invalid_json"); // bool coord
  assert.equal(reason('{"kind": "slice", "start": [0]}'), "invalid_json"); // missing stop
  assert.equal(reason('{"kind": "box", "inclusive_min": 5}'), "invalid_json"); // non-list bound
  assert.equal(reason('{"kind": "points", "coords": [[true]]}'), "invalid_json"); // bool coord
  assert.equal(reason('{"kind": "transform", "output": 5}'), "invalid_json"); // non-list output
});
```

- [ ] **Step 2: Run the tests**

Run: `node --test test/parse.test.ts 2>&1 | tail -5`
Expected: both tests pass (behavior already implemented). If any fail, reconcile the implementation rather than weakening the test.

- [ ] **Step 3: Run the full unit suite**

Run: `node --test test/*.test.ts 2>&1 | tail -8`
Expected: all unit test files pass.

- [ ] **Step 4: Commit**

```bash
git add typescript/test/parse.test.ts
git commit -m "test(ts): parse, dispatch, and malformed-input validation"
```

---

## Task 12: Conformance runner

**Files:**
- Create: `typescript/test/conformance.test.ts`

Runs the **same** `/conformance/` corpus (29 fixtures) the Rust and Python references pass: schema-validate every success input (via `ajv`), normalize, compare to the fixture's `normalized` with `deepStrictEqual`; for error cases, assert `NdsqError` with the stated reason code.

- [ ] **Step 1: Write the runner**

Create `typescript/test/conformance.test.ts`:

```typescript
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import Ajv2020 from "ajv/dist/2020.js";
import { NdsqError, normalize, parse } from "../src/index.ts";

const repoRoot = join(import.meta.dirname, "..", "..");
const corpusDir = join(repoRoot, "conformance");
const schemaPath = join(repoRoot, "schema", "ndsq.schema.json");

// strict:false so ajv accepts the schema's `additionalProperties:false` alongside
// `allOf`/`$ref` (a strict-mode warning), matching the lenient Rust/Python validators.
const ajv = new Ajv2020({ strict: false });
const validate = ajv.compile(JSON.parse(readFileSync(schemaPath, "utf8")));

interface Fixture {
  name: string;
  input: unknown;
  normalized?: unknown;
  error?: string;
}

const fixtures: Fixture[] = [];
for (const file of readdirSync(corpusDir).filter((f) => f.endsWith(".json")).sort()) {
  for (const c of JSON.parse(readFileSync(join(corpusDir, file), "utf8")) as Fixture[]) {
    fixtures.push(c);
  }
}

test("corpus is populated", () => {
  assert.ok(fixtures.length >= 15, `expected a populated corpus, found ${fixtures.length}`);
});

for (const fixture of fixtures) {
  test(`corpus: ${fixture.name}`, () => {
    const input = JSON.stringify(fixture.input);
    if (fixture.error !== undefined) {
      // Error inputs may be intentionally schema-invalid; not schema-checked.
      try {
        normalize(parse(input));
        assert.fail(`${fixture.name}: expected an error`);
      } catch (e) {
        assert.ok(e instanceof NdsqError, `${fixture.name}: ${String(e)}`);
        assert.equal((e as NdsqError).reason, fixture.error);
      }
    } else {
      assert.ok(validate(fixture.input), `${fixture.name}: input fails schema`);
      assert.deepStrictEqual(normalize(parse(input)), fixture.normalized);
    }
  });
}
```

- [ ] **Step 2: Run the runner**

Run: `node --test test/conformance.test.ts 2>&1 | tail -10`
Expected: PASS — one test per fixture (29 currently) plus the populated check, all green. **If a success case's `deepStrictEqual` fails, it is a TypeScript bug** — fix the TS, never the fixture; report which case and the diff.

Note on the `ajv` import: the runner uses the default import `import Ajv2020 from "ajv/dist/2020.js"` (ajv v8's documented form). If that fails to construct under your installed ajv/Node ESM-CJS interop, try the named form `import { Ajv2020 } from "ajv/dist/2020.js"`, or `import _Ajv2020 from "ajv/dist/2020.js"; const Ajv2020 = (_Ajv2020 as { default?: unknown }).default ?? _Ajv2020;`. Adjust only this import line; do not change the test logic.

- [ ] **Step 3: Run the full suite**

Run: `node --test test/*.test.ts 2>&1 | tail -8`
Expected: all unit tests + every conformance case pass.

- [ ] **Step 4: Commit**

```bash
git add typescript/test/conformance.test.ts
git commit -m "test(ts): conformance runner against the shared corpus"
```

---

## Task 13: Type-check, README, final verification

**Files:**
- Create: `typescript/README.md`

- [ ] **Step 1: Type-check the whole package**

Run: `npx tsc --noEmit 2>&1 | tail -10`
Expected: no errors. Fix any type errors surfaced (the source uses strict mode + `verbatimModuleSyntax`; all imports must use `.ts` extensions and `import type` for type-only imports).

- [ ] **Step 2: Write the package README**

Create `typescript/README.md` with: one paragraph (the TypeScript peer implementation, passing the shared conformance corpus); the hybrid model (snake_case wire types mirroring the JSON, camelCase builder functions for ergonomic construction — note the zarrita.js precedent); a usage example:

```typescript
import { normalize, parse, box } from "ndsq";

// from JSON
normalize(parse('{"kind": "slice", "start": [0], "stop": [10], "step": [2]}'));

// from a camelCase builder
normalize(box({ inclusiveMin: [0, 0], exclusiveMax: [3, 4] }));
```

a note that it adapts tensorstore's index model (link `../spec/ndsq.md`) and is validated against `../conformance/`; and a "Develop" note: `npm test` (unit + conformance) and `npm run typecheck`. Mention it requires Node ≥ 23.6 (runs TypeScript natively).

- [ ] **Step 3: Full verification**

Run: `node --test test/*.test.ts 2>&1 | tail -8`
Expected: every test passes (unit + all conformance cases).

Run: `node --input-type=module -e "import { normalize, box } from './src/index.ts'; console.log(JSON.stringify(normalize(box({ shape: [3] }))))" 2>&1 | tail -2`
Expected: prints `{"input_rank":1,"input_inclusive_min":[0],"input_exclusive_max":[3],"input_labels":[""],"output":[{"offset":0,"stride":1,"input_dimension":0}]}`.

- [ ] **Step 4: Commit**

```bash
git add typescript/README.md
git commit -m "docs(ts): package README with usage and pointers"
```

---

## Self-review checklist (run after implementation)

- **Spec coverage:** every desugaring has a task and is exercised by the shared corpus (Task 12): point/box=Task 8, slice=Task 9, points=Task 10, transform=Task 7. Errors: Reason codes=Task 2; step/negative/rank in slice=Task 9; multiple_upper_bounds/rank in domain=Task 5; unknown_kind/invalid_json in parse + validators=Tasks 8/11.
- **Cross-impl parity:** the TS normalizes to the SAME canonical bodies as Rust/Python (same corpus, Task 12), including the 5 structural-malformed → invalid_json fixtures (validators built in from Task 3). `box` accepts implicit/inf bounds (Task 5 test); `slice` integer-only.
- **Type consistency:** `parse`, `normalize`, `canonicalizeDomain`, `canonicalizeTransform`, `canonicalizeOutputMap`, `identityOutput`, `parseBound`/`boundToJSON`, `require*`, `desugar*` are used with identical signatures across tasks. `Reason` member names (`StepZero`, etc.) map to the wire strings.
- **No placeholders:** the only thrown stubs are the Task 8 `desugarSlice`/`desugarPoints`, replaced in Tasks 9–10.
- **Node-native TS constraints honored:** no `enum` (const object for `Reason`), no namespaces/param-properties; `.ts` extensions on relative imports; `import type` for type-only imports under `verbatimModuleSyntax`.
- **Known limitation (documented, deferred):** JS numbers are IEEE-754 doubles, so index values beyond 2^53 lose precision — consistent with the spec's i64-range note (§3.5); not exercised by the corpus.
