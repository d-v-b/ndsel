import { test } from "node:test";
import assert from "node:assert/strict";
import { NdselError } from "../src/errors.ts";
import { normalize, parse } from "../src/index.ts";

function norm(text: string): Record<string, unknown> {
  return normalize(parse(text)) as unknown as Record<string, unknown>;
}

function reason(text: string): string {
  try {
    normalize(parse(text));
  } catch (e) {
    if (e instanceof NdselError) return e.reason;
  }
  throw new Error("expected NdselError");
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

test("negative start uses truncating division", () => {
  // x[-3:3:2] selects -3,-1,1; o=trunc(-3/2)=-1, offset=-3-2*(-1)=-1, domain [-1,2)
  const d = norm('{"kind": "slice", "start": [-3], "stop": [3], "step": [2]}');
  assert.deepStrictEqual(d.input_inclusive_min, [-1]);
  assert.deepStrictEqual(d.input_exclusive_max, [2]);
  assert.deepStrictEqual(d.output, [{ offset: -1, stride: 2, input_dimension: 0 }]);
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

// Negative steps desugar by the same rule as positive ones (spec 5.3):
// [start, stop, step, inclusive_min, exclusive_max, offset]
const negativeStepCases: [number, number, number, number, number, number][] = [
  [19, -1, -1, -19, 1, 0], // x[19:-1:-1] — a reversed length-20 axis: points 19..0
  [15, 5, -2, -7, -2, 1], // x[15:5:-2]  — divisible span: points 15,13,11,9,7
  [15, 5, -4, -3, 0, 3], // x[15:5:-4]  — span 10 is not divisible by 4: points 15,11,7
  [-1, -6, -2, 0, 3, -1], // x[-1:-6:-2] — negative interval; trunc(-1/-2)=0, so origin 0
  [5, 4, -3, -1, 0, 2], // x[5:4:-3]   — one point
  [5, 5, -1, -5, -5, 0], // x[5:5:-1]   — empty is legal anywhere, at origin trunc(5/-1)=-5
];

test("negative step slices", () => {
  for (const [start, stop, step, lo, hi, offset] of negativeStepCases) {
    const label = `${start}:${stop}:${step}`;
    const d = norm(`{"kind": "slice", "start": [${start}], "stop": [${stop}], "step": [${step}]}`);
    assert.deepStrictEqual(d.input_inclusive_min, [lo], label);
    assert.deepStrictEqual(d.input_exclusive_max, [hi], label);
    assert.deepStrictEqual(d.output, [{ offset, stride: step, input_dimension: 0 }], label);
  }
});

test("negative step keeps labels", () => {
  const d = norm('{"kind": "slice", "start": [19], "stop": [-1], "step": [-1], "labels": ["x"]}');
  assert.deepStrictEqual(d.input_labels, ["x"]);
});

test("reversed interval with a positive step is an error", () => {
  assert.equal(reason('{"kind": "slice", "start": [9], "stop": [0], "step": [2]}'), "bounds_out_of_order");
});

test("reversed interval with a negative step is an error", () => {
  assert.equal(reason('{"kind": "slice", "start": [5], "stop": [6], "step": [-1]}'), "bounds_out_of_order");
});

test("errors", () => {
  assert.equal(reason('{"kind": "slice", "start": [0], "stop": [4], "step": [0]}'), "step_zero");
  assert.equal(reason('{"kind": "slice", "start": [0, 0], "stop": [4]}'), "rank_mismatch");
});
