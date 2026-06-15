import { test } from "node:test";
import assert from "node:assert/strict";
import { NdselError } from "../src/errors.ts";
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
    (e) => e instanceof NdselError && e.reason === "rank_mismatch",
  );
});
