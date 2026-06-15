import { canonicalizeDomain } from "./domain.ts";
import type { BoxMessage, PointMessage, PointsMessage, SliceMessage } from "./messages.ts";
import type { OutputMapJson } from "./output.ts";
import { type Transform, identityOutput } from "./transform.ts";
import { requireIntArray } from "./values.ts";

export function desugarPoint(msg: PointMessage): Transform {
  const coords = requireIntArray((msg as { coords?: unknown }).coords, "point.coords");
  return {
    input_rank: 0,
    input_inclusive_min: [],
    input_exclusive_max: [],
    input_labels: [],
    output: coords.map((c) => ({ offset: c })),
  };
}

export function desugarBox(msg: BoxMessage): Transform {
  const domain = canonicalizeDomain({
    inclusive_min: msg.inclusive_min,
    exclusive_max: msg.exclusive_max,
    inclusive_max: msg.inclusive_max,
    shape: msg.shape,
    labels: msg.labels,
  });
  return { ...domain, output: identityOutput(domain.input_rank) };
}

export function desugarSlice(_msg: SliceMessage): Transform {
  throw new Error("desugarSlice — implemented in Task 9");
}

export function desugarPoints(_msg: PointsMessage): Transform {
  throw new Error("desugarPoints — implemented in Task 10");
}

export type { OutputMapJson };
