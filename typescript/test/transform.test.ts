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
