import { NdsqError, Reason } from "./errors.ts";
import { canonicalizeDomain } from "./domain.ts";
import type { BoxMessage, PointMessage, PointsMessage, SliceMessage } from "./messages.ts";
import type { OutputMapJson } from "./output.ts";
import { type Transform, identityOutput } from "./transform.ts";
import {
  type BoundJson,
  demote,
  floorDivBig,
  requireArray,
  requireIntArray,
  requireStringArray,
  toBig,
} from "./values.ts";

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

export function desugarSlice(msg: SliceMessage): Transform {
  const m = msg as { start?: unknown; stop?: unknown; step?: unknown; labels?: unknown };
  const start = requireIntArray(m.start, "slice.start");
  const stop = requireIntArray(m.stop, "slice.stop");
  const rank = start.length;
  if (stop.length !== rank) {
    throw new NdsqError(Reason.RankMismatch, "start and stop must have equal length");
  }
  let step = start.map(() => 1n);
  if (m.step !== undefined) {
    const raw = requireIntArray(m.step, "slice.step");
    if (raw.length !== rank) {
      throw new NdsqError(Reason.RankMismatch, "step length must match start/stop");
    }
    step = raw.map(toBig);
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
    const a = toBig(start[k]);
    const b = toBig(stop[k]);
    const s = step[k];
    if (s === 0n) throw new NdsqError(Reason.StepZero, "step must be non-zero");
    if (s < 0n) throw new NdsqError(Reason.NegativeStepUnsupported, "negative step is not yet specified");
    const count = b <= a ? 0n : (b - a + s - 1n) / s; // ceil((b-a)/s) for s>0
    const o = floorDivBig(a, s); // floor(a/s) for s > 0
    const offset = a - s * o; // lattice phase in [0, s)
    inclusiveMin.push(demote(o));
    exclusiveMax.push(demote(o + count));
    output.push({ offset: demote(offset), stride: demote(s), input_dimension: k });
  }

  return {
    input_rank: rank,
    input_inclusive_min: inclusiveMin,
    input_exclusive_max: exclusiveMax,
    input_labels: labels ?? Array.from({ length: rank }, () => ""),
    output,
  };
}

export function desugarPoints(msg: PointsMessage): Transform {
  const rawCoords = requireArray((msg as { coords?: unknown }).coords, "points.coords");
  const coords = rawCoords.map((p, i) => requireIntArray(p, `points.coords[${i}]`));
  const m = coords.length;
  const n = coords.length > 0 ? coords[0].length : 0;
  for (const p of coords) {
    if (p.length !== n) {
      throw new NdsqError(Reason.RankMismatch, "all points must have equal dimensionality");
    }
  }

  const output: OutputMapJson[] = [];
  for (let k = 0; k < n; k++) {
    const column = coords.map((p) => p[k]);
    output.push({ offset: 0, stride: 1, index_array: column, index_array_bounds: ["-inf", "+inf"] });
  }
  return {
    input_rank: 1,
    input_inclusive_min: [0],
    input_exclusive_max: [m],
    input_labels: [""],
    output,
  };
}

