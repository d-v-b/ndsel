import { test } from "node:test";
import assert from "node:assert/strict";
import { NdsqError } from "../src/errors.ts";
import { normalize, parse } from "../src/index.ts";

function reason(text: string): string {
  try {
    normalize(parse(text));
  } catch (e) {
    if (e instanceof NdsqError) return e.reason;
  }
  throw new Error("expected NdsqError");
}

test("kind dispatch errors", () => {
  assert.equal(reason('{"kind": "bogus"}'), "unknown_kind");
  assert.equal(reason('{"coords": [1]}'), "invalid_json"); // missing kind
  assert.equal(reason("{ not json"), "invalid_json");
  assert.equal(reason("[1, 2, 3]"), "invalid_json"); // non-object
});

test("malformed bodies are invalid_json (Rust/Python parity)", () => {
  assert.equal(reason('{"kind": "point"}'), "invalid_json"); // missing coords
  assert.equal(reason('{"kind": "point", "coords": [true]}'), "invalid_json"); // bool coord
  assert.equal(reason('{"kind": "slice", "start": [0]}'), "invalid_json"); // missing stop
  assert.equal(reason('{"kind": "box", "inclusive_min": 5}'), "invalid_json"); // non-list bound
  assert.equal(reason('{"kind": "points", "coords": [[true]]}'), "invalid_json"); // bool coord
  assert.equal(reason('{"kind": "transform", "output": 5}'), "invalid_json"); // non-list output
});
