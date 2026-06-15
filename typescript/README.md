# ndsel — TypeScript

The TypeScript peer implementation of **ndsel** (a JSON-serializable representation of n-dimensional array indexing / "spatial query"), passing the same shared conformance corpus as the Rust and Python references.

## Design: hybrid snake_case wire + camelCase builders

**Wire types** use snake_case that mirrors the JSON byte-for-byte — `Message` (the top-level union), `Transform`, and their fields (`input_inclusive_min`, `input_exclusive_max`, etc.) match the schema exactly with no renaming.

**Builder functions** use camelCase for ergonomic construction: `point`, `box`, `slice`, `points`. This mirrors the [zarrita.js](https://github.com/zarrita-dev/zarrita.js) precedent where metadata structures are snake_case (matching the Zarr spec) while constructor options are camelCase.

## Usage

```typescript
import { normalize, parse, box } from "ndsel";

// from JSON
normalize(parse('{"kind": "slice", "start": [0], "stop": [10], "step": [2]}'));

// from a camelCase builder
normalize(box({ inclusiveMin: [0, 0], exclusiveMax: [3, 4] }));
```

The `normalize` function expands the input into a canonical `Transform` (an explicit rank, label, and per-output-dimension stride/offset/input-dimension mapping — equivalent to a TensorStore `IndexTransform` body):

```
normalize(box({ shape: [3] }))
// → {"input_rank":1,"input_inclusive_min":[0],"input_exclusive_max":[3],"input_labels":[""],"output":[{"offset":0,"stride":1,"input_dimension":0}]}
```

## Specification and conformance

The index model adapts [TensorStore's index transform](../spec/ndsel.md). The implementation is validated against the shared corpus in [`../conformance/`](../conformance/).

## Develop

```bash
npm test        # unit tests + all conformance cases
npm run typecheck
```

Requires **Node ≥ 23.6**: TypeScript source files run natively — no build step.

> **Integer range (`number | bigint`).** Index values cover the full 64-bit signed range (`spec §3.5`). An integer within JavaScript's safe range (`±(2^53 − 1)`) is a `number`; one beyond it is a `bigint` — so `parse` reads large integer literals losslessly and `normalize` returns `bigint` for them. The `Int = number | bigint` distinction is magnitude-based, so the canonical form is representation-independent. Builders accept either (`box({ exclusiveMax: [3, 4] })` or `[3n, 4n]`). Use the exported `stringifyJson` to serialize a result, since `JSON.stringify` throws on `bigint`.
