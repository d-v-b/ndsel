# ndsq — TypeScript

The TypeScript peer implementation of **ndsq** (a JSON-serializable representation of n-dimensional array indexing / "spatial query"), passing the same shared conformance corpus as the Rust and Python references.

## Design: hybrid snake_case wire + camelCase builders

**Wire types** use snake_case that mirrors the JSON byte-for-byte — `Message` (the top-level union), `Transform`, and their fields (`input_inclusive_min`, `input_exclusive_max`, etc.) match the schema exactly with no renaming.

**Builder functions** use camelCase for ergonomic construction: `point`, `box`, `slice`, `points`. This mirrors the [zarrita.js](https://github.com/zarrita-dev/zarrita.js) precedent where metadata structures are snake_case (matching the Zarr spec) while constructor options are camelCase.

## Usage

```typescript
import { normalize, parse, box } from "ndsq";

// from JSON
normalize(parse('{"kind": "slice", "start": [0], "stop": [10], "step": [2]}'));

// from a camelCase builder
normalize(box({ inclusiveMin: [0, 0], exclusiveMax: [3, 4] }));
```

The `normalize` function expands the input into a canonical `IndexTransform` (an explicit rank, label, and per-output-dimension stride/offset/input-dimension mapping):

```
normalize(box({ shape: [3] }))
// → {"input_rank":1,"input_inclusive_min":[0],"input_exclusive_max":[3],"input_labels":[""],"output":[{"offset":0,"stride":1,"input_dimension":0}]}
```

## Specification and conformance

The index model adapts [tensorstore's index transform](../spec/ndsq.md). The implementation is validated against the shared corpus in [`../conformance/`](../conformance/).

## Develop

```bash
npm test        # unit tests + all conformance cases
npm run typecheck
```

Requires **Node ≥ 23.6**: TypeScript source files run natively — no build step.
