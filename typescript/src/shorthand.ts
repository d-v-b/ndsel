import { NdsqError, Reason } from "./errors.ts";
import { canonicalizeDomain } from "./domain.ts";
import type { BoxMessage, PointMessage, PointsMessage, SliceMessage } from "./messages.ts";
import type { OutputMapJson } from "./output.ts";
import { type Transform, identityOutput } from "./transform.ts";
import { type BoundJson, requireIntArray, requireStringArray } from "./values.ts";

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

function ceilDiv(p: number, q: number): number {
  // Ceiling of p/q for p >= 0, q > 0.
  return Math.floor((p + q - 1) / q);
}

export function desugarSlice(msg: SliceMessage): Transform {
  const m = msg as { start?: unknown; stop?: unknown; step?: unknown; labels?: unknown };
  const start = requireIntArray(m.start, "slice.start");
  const stop = requireIntArray(m.stop, "slice.stop");
  const rank = start.length;
  if (stop.length !== rank) {
    throw new NdsqError(Reason.RankMismatch, "start and stop must have equal length");
  }
  let step: number[];
  if (m.step === undefined) {
    step = Array.from({ length: rank }, () => 1);
  } else {
    step = requireIntArray(m.step, "slice.step");
    if (step.length !== rank) {
      throw new NdsqError(Reason.RankMismatch, "step length must match start/stop");
    }
  }
  let labels: string[] | null = null;
  if (m.labels !== undefined) {
    labels = requireStringArray(m.labels, "slice.labels");
    if (labels.length !== rank) {
      throw new NdsqError(Reason.RankMismatch, "labels length must match start/stop");
    }
  }

  const inclusiveMin: BoundJson[] = [];
  const exclusiveMax: BoundJson[] = [];
  const output: OutputMapJson[] = [];
  for (let k = 0; k < rank; k++) {
    const a = start[k];
    const b = stop[k];
    const s = step[k];
    if (s === 0) throw new NdsqError(Reason.StepZero, "step must be non-zero");
    if (s < 0) throw new NdsqError(Reason.NegativeStepUnsupported, "negative step is not yet specified");
    const count = b <= a ? 0 : ceilDiv(b - a, s);
    const o = Math.floor(a / s); // floor(a/s) for s > 0
    const offset = a - s * o; // lattice phase in [0, s)
    inclusiveMin.push(o);
    exclusiveMax.push(o + count);
    output.push({ offset, stride: s, input_dimension: k });
  }

  return {
    input_rank: rank,
    input_inclusive_min: inclusiveMin,
    input_exclusive_max: exclusiveMax,
    input_labels: labels ?? Array.from({ length: rank }, () => ""),
    output,
  };
}

export function desugarPoints(_msg: PointsMessage): Transform {
  throw new Error("desugarPoints — implemented in Task 10");
}

