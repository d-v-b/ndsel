import { test } from "node:test";
import assert from "node:assert/strict";
import { NdsqError, Reason } from "../src/errors.ts";

test("reason codes are stable", () => {
  assert.equal(Reason.StepZero, "step_zero");
  assert.equal(Reason.NegativeStepUnsupported, "negative_step_unsupported");
  assert.equal(Reason.MultipleUpperBounds, "multiple_upper_bounds");
  assert.equal(Reason.RankMismatch, "rank_mismatch");
  assert.equal(Reason.UnknownKind, "unknown_kind");
  assert.equal(Reason.InvalidJson, "invalid_json");
});

test("error carries reason and detail", () => {
  const err = new NdsqError(Reason.StepZero, "step must be non-zero");
  assert.equal(err.reason, "step_zero");
  assert.ok(err instanceof NdsqError);
  assert.ok(err instanceof Error);
  assert.match(err.message, /step must be non-zero/);
});
