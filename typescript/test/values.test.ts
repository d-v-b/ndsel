import { test } from "node:test";
import assert from "node:assert/strict";
import { NdselError } from "../src/errors.ts";
import { boundToJSON, parseBound, parseIndexValue, requireInt, requireIntArray, requireStringArray } from "../src/values.ts";

test("parseIndexValue accepts ints and sentinels", () => {
  assert.equal(parseIndexValue(7), 7);
  assert.equal(parseIndexValue("-inf"), "-inf");
  assert.equal(parseIndexValue("+inf"), "+inf");
});

test("parseIndexValue rejects bool, float, and garbage", () => {
  assert.throws(() => parseIndexValue(true), NdselError);
  assert.throws(() => parseIndexValue(1.5), NdselError);
  assert.throws(() => parseIndexValue("nope"), NdselError);
});

test("bare value is explicit, bracket is implicit", () => {
  assert.deepEqual(parseBound(7), { value: 7, implicit: false });
  assert.deepEqual(parseBound([7]), { value: 7, implicit: true });
  assert.deepEqual(parseBound(["-inf"]), { value: "-inf", implicit: true });
});

test("multi-element bracket is invalid", () => {
  assert.throws(() => parseBound([1, 2]), NdselError);
});

test("boundToJSON round-trips explicit and implicit", () => {
  assert.equal(boundToJSON({ value: 7, implicit: false }), 7);
  assert.deepEqual(boundToJSON({ value: 7, implicit: true }), [7]);
  assert.deepEqual(boundToJSON({ value: "-inf", implicit: true }), ["-inf"]);
});

test("require validators", () => {
  assert.equal(requireInt(3, "x"), 3);
  assert.throws(() => requireInt(true, "x"), NdselError);
  assert.deepEqual(requireIntArray([1, 2], "x"), [1, 2]);
  assert.throws(() => requireIntArray(5, "x"), NdselError);
  assert.throws(() => requireIntArray([true], "x"), NdselError);
  assert.deepEqual(requireStringArray(["a", ""], "x"), ["a", ""]);
  assert.throws(() => requireStringArray([1], "x"), NdselError);
});
