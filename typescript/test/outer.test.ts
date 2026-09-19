import { test } from "node:test";
import assert from "node:assert/strict";
import { NdselError } from "../src/errors.ts";
import { normalize, outer, parse } from "../src/index.ts";

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

const ia = (index_array: unknown) => ({ offset: 0, stride: 1, index_array, index_array_bounds: ["-inf", "+inf"] });
const id = (k: number) => ({ offset: 0, stride: 1, input_dimension: k });

test("outer desugaring", () => {
  // [axes, input_inclusive_min, input_exclusive_max, output]
  const cases: [string, unknown, unknown, unknown][] = [
    // The whole axis: implicit infinite bounds, identity map.
    ["[null]", [["-inf"]], [["+inf"]], [id(0)]],
    // A range is one dimension of a `slice`: x[5:10:2] keeps its source frame.
    ['[{"start": 5, "stop": 10, "step": 2}]', [2], [5], [{ offset: 1, stride: 2, input_dimension: 0 }]],
    // Intervals expand in order; the result is numbered [0, m).
    ['[{"intervals": [[0, 2], [5, 7]]}]', [0], [4], [ia([0, 1, 5, 6])]],
    // The array broadcasts: m along its own dimension, 1 along every other.
    [
      '[null, {"indices": [4, 2]}, null]',
      [["-inf"], 0, ["-inf"]],
      [["+inf"], 2, ["+inf"]],
      [id(0), ia([[[4], [2]]]), id(2)],
    ],
    // One coordinate collapses to a constant map; none collapses to offset 0.
    ['[{"indices": [7]}]', [0], [1], [{ offset: 7 }]],
    ['[{"intervals": [[3, 3]]}]', [0], [0], [{ offset: 0 }]],
  ];
  for (const [axes, lo, hi, output] of cases) {
    const d = norm(`{"kind": "outer", "axes": ${axes}}`);
    assert.deepStrictEqual(d.input_inclusive_min, lo, axes);
    assert.deepStrictEqual(d.input_exclusive_max, hi, axes);
    assert.deepStrictEqual(d.output, output, axes);
  }
});

test("outer builder matches parse", () => {
  assert.deepStrictEqual(
    normalize(outer([null, { intervals: [[0, 3]] }], ["y", "x"])),
    normalize(parse('{"kind": "outer", "axes": [null, {"intervals": [[0, 3]]}], "labels": ["y", "x"]}')),
  );
});

test("selector with members of two forms is unknown_field", () => {
  assert.equal(reason('{"kind": "outer", "axes": [{"indices": [1], "intervals": [[0, 1]]}]}'), "unknown_field");
});

test("range missing stop is invalid_json", () => {
  assert.equal(reason('{"kind": "outer", "axes": [{"start": 0}]}'), "invalid_json");
});

test("interval that is not a pair is invalid_json", () => {
  assert.equal(reason('{"kind": "outer", "axes": [{"intervals": [[1, 2, 3]]}]}'), "invalid_json");
});

test("reversed interval is bounds_out_of_order", () => {
  assert.equal(reason('{"kind": "outer", "axes": [{"intervals": [[5, 3]]}]}'), "bounds_out_of_order");
});

test("range step zero", () => {
  assert.equal(reason('{"kind": "outer", "axes": [{"start": 0, "stop": 4, "step": 0}]}'), "step_zero");
});

test("labels length mismatch is rank_mismatch", () => {
  assert.equal(reason('{"kind": "outer", "axes": [null, null], "labels": ["x"]}'), "rank_mismatch");
});
