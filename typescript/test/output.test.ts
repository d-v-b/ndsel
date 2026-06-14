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
