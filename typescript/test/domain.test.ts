import { test } from "node:test";
import assert from "node:assert/strict";
import { NdselError } from "../src/errors.ts";
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
  assert.throws(() => canonicalizeDomain({ shape: [3], exclusive_max: [3] }), (e) => e instanceof NdselError && e.reason === "multiple_upper_bounds");
});

test("length disagreement is rank_mismatch", () => {
  assert.throws(() => canonicalizeDomain({ inclusive_min: [0, 0], shape: [3] }), (e) => e instanceof NdselError && e.reason === "rank_mismatch");
});

test("non-list bound is invalid_json", () => {
  assert.throws(() => canonicalizeDomain({ inclusive_min: 5 }), (e) => e instanceof NdselError && e.reason === "invalid_json");
});
