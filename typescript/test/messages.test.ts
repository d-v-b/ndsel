import { test } from "node:test";
import assert from "node:assert/strict";
import type { BoxMessage, Message, SliceMessage } from "../src/messages.ts";

test("message literals are typed against the union", () => {
  // Compile-time checks; the assertions just ensure the file is exercised at runtime.
  const box: BoxMessage = { kind: "box", inclusive_min: [0, 0], exclusive_max: [3, 4] };
  const slice: SliceMessage = { kind: "slice", start: [0], stop: [10], step: [2] };
  const messages: Message[] = [box, slice, { kind: "point", coords: [1] }];
  assert.equal(box.kind, "box");
  assert.equal(slice.step?.[0], 2);
  assert.equal(messages.length, 3);
});
