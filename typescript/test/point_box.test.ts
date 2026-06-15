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
  const d = normalize(box({ inclusiveMin: [0, 0], exclusiveMax: [3, 4] }));
  assert.deepStrictEqual(d.input_exclusive_max, [3, 4]);
});
