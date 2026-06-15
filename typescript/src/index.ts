/**
 * ndsel — JSON-serializable n-dimensional spatial queries.
 *
 * Parse a JSON string with `parse`, then reduce it to a canonical `Transform`
 * with `normalize`. Construct messages ergonomically with the camelCase
 * builders (`point`/`box`/`slice`/`points`).
 */

import { NdsqError, Reason } from "./errors.ts";
import type { Message } from "./messages.ts";
import { desugarBox, desugarPoint, desugarPoints, desugarSlice } from "./shorthand.ts";
import { type Transform, canonicalizeTransform } from "./transform.ts";

export { NdsqError, Reason };
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
export type { BoundJson, IndexValue } from "./values.ts";
export { type BoxOptions, type SliceOptions, box, point, points, slice } from "./builders.ts";

const KNOWN_KINDS = new Set(["point", "box", "slice", "points", "transform"]);

/** Parse a JSON string and validate the `kind` discriminator. */
export function parse(text: string): Message {
  let obj: unknown;
  try {
    obj = JSON.parse(text);
  } catch (err) {
    throw new NdsqError(Reason.InvalidJson, err instanceof Error ? err.message : String(err));
  }
  if (typeof obj !== "object" || obj === null || Array.isArray(obj)) {
    throw new NdsqError(Reason.InvalidJson, "message must be a JSON object");
  }
  const kind = (obj as { kind?: unknown }).kind;
  if (typeof kind !== "string") {
    throw new NdsqError(Reason.InvalidJson, "missing string `kind`");
  }
  if (!KNOWN_KINDS.has(kind)) {
    throw new NdsqError(Reason.UnknownKind, `unknown kind: ${kind}`);
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
      throw new NdsqError(Reason.UnknownKind, `unknown kind: ${String(k)}`);
    }
  }
}
