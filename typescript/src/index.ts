/**
 * ndsel — JSON-serializable n-dimensional spatial queries.
 *
 * Parse a JSON string with `parse`, then reduce it to a canonical `Transform`
 * with `normalize`. Construct messages ergonomically with the camelCase
 * builders (`point`/`box`/`slice`/`points`).
 */

import { NdselError, Reason } from "./errors.ts";
import { parseJson, stringifyJson } from "./json.ts";
import type { Message } from "./messages.ts";
import { desugarBox, desugarPoint, desugarPoints, desugarSlice } from "./shorthand.ts";
import { type Transform, canonicalizeTransform } from "./transform.ts";

export { NdselError, Reason };
export { stringifyJson };
export type { ReasonCode } from "./errors.ts";
export type {
  BoxMessage,
  Message,
  OutputMapInput,
  PointMessage,
  PointsMessage,
  SliceMessage,
  TransformMessage,
} from "./messages.ts";
export type { Transform } from "./transform.ts";
export type { OutputMapJson } from "./output.ts";
export type { DomainFields } from "./domain.ts";
export type { BoundJson, IndexValue, Int } from "./values.ts";
export { type BoxOptions, type SliceOptions, box, point, points, slice } from "./builders.ts";

const KNOWN_KINDS = new Set(["point", "box", "slice", "points", "transform"]);

// The complete set of recognized members per kind (`kind` included). Strict:
// any other member is rejected with `unknown_field`.
const ALLOWED_FIELDS: Record<string, Set<string>> = {
  point: new Set(["kind", "coords"]),
  box: new Set(["kind", "inclusive_min", "exclusive_max", "inclusive_max", "shape", "labels"]),
  slice: new Set(["kind", "start", "stop", "step", "labels"]),
  points: new Set(["kind", "coords"]),
  transform: new Set([
    "kind",
    "input_rank",
    "input_inclusive_min",
    "input_exclusive_max",
    "input_inclusive_max",
    "input_shape",
    "input_labels",
    "output",
  ]),
};

/**
 * Parse a JSON string and validate the `kind` discriminator. Integers outside
 * the JS safe range are parsed losslessly as `bigint` (§3.5).
 */
export function parse(text: string): Message {
  let obj: unknown;
  try {
    obj = parseJson(text);
  } catch (err) {
    throw new NdselError(Reason.InvalidJson, err instanceof Error ? err.message : String(err));
  }
  if (typeof obj !== "object" || obj === null || Array.isArray(obj)) {
    throw new NdselError(Reason.InvalidJson, "message must be a JSON object");
  }
  const kind = (obj as { kind?: unknown }).kind;
  if (typeof kind !== "string") {
    throw new NdselError(Reason.InvalidJson, "missing string `kind`");
  }
  if (!KNOWN_KINDS.has(kind)) {
    throw new NdselError(Reason.UnknownKind, `unknown kind: ${kind}`);
  }
  for (const key of Object.keys(obj)) {
    if (!ALLOWED_FIELDS[kind].has(key)) {
      throw new NdselError(Reason.UnknownField, `unrecognized field for kind ${kind}: ${key}`);
    }
  }
  return obj as Message;
}

/** Reduce a message to its canonical Transform. */
export function normalize(message: Message): Transform {
  switch (message.kind) {
    case "point":
      return desugarPoint(message);
    case "box":
      return desugarBox(message);
    case "slice":
      return desugarSlice(message);
    case "points":
      return desugarPoints(message);
    case "transform":
      return canonicalizeTransform(message);
    default: {
      const k = (message as { kind?: unknown }).kind;
      throw new NdselError(Reason.UnknownKind, `unknown kind: ${String(k)}`);
    }
  }
}
