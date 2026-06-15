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
