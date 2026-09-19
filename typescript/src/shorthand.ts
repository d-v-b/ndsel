import { NdselError, Reason } from "./errors.ts";
import { canonicalizeDomain } from "./domain.ts";
import type { BoxMessage, OuterMessage, PointMessage, PointsMessage, SliceMessage } from "./messages.ts";
import type { OutputMapJson } from "./output.ts";
import { type Transform, identityOutput } from "./transform.ts";
import {
  type BoundJson,
  type Int,
  demote,
  requireArray,
  requireInt,
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

/** One dimension of the `slice` desugaring (spec 5.3): its domain bounds and map. */
function sliceDimension(a: bigint, b: bigint, s: bigint, k: number): [BoundJson, BoundJson, OutputMapJson] {
  if (s === 0n) throw new NdselError(Reason.StepZero, "step must be non-zero");
  // Source interval [a, b) for s > 0, [b+1, a+1) for s < 0 — see spec 5.3.
  const n = s > 0n ? b - a : a - b;
  if (n < 0n) {
    throw new NdselError(
      Reason.BoundsOutOfOrder,
      `dimension ${k}: stop must be at or after start in the direction of travel ` +
        `(stop >= start for step > 0, stop <= start for step < 0); ` +
        `got start=${a}, stop=${b}, step=${s}`,
    );
  }
  const abs = s < 0n ? -s : s;
  const count = (n + abs - 1n) / abs; // ceil(n/|s|), n >= 0
  const o = a / s; // trunc(a/s), both signs: BigInt `/` truncates toward zero (spec 5.3)
  const offset = a - s * o; // lattice phase: |offset| < |s|, sign of a
  return [demote(o), demote(o + count), { offset: demote(offset), stride: demote(s), input_dimension: k }];
}

export function desugarSlice(msg: SliceMessage): Transform {
  const m = msg as { start?: unknown; stop?: unknown; step?: unknown; labels?: unknown };
  const start = requireIntArray(m.start, "slice.start");
  const stop = requireIntArray(m.stop, "slice.stop");
  const rank = start.length;
  if (stop.length !== rank) {
    throw new NdselError(Reason.RankMismatch, "start and stop must have equal length");
  }
  let step = start.map(() => 1n);
  if (m.step !== undefined) {
    const raw = requireIntArray(m.step, "slice.step");
    if (raw.length !== rank) {
      throw new NdselError(Reason.RankMismatch, "step length must match start/stop");
    }
    step = raw.map(toBig);
  }
  let labels: string[] | null = null;
  if (m.labels !== undefined) {
    labels = requireStringArray(m.labels, "slice.labels");
    if (labels.length !== rank) {
      throw new NdselError(Reason.RankMismatch, "labels length must match start/stop");
    }
  }

  const inclusiveMin: BoundJson[] = [];
  const exclusiveMax: BoundJson[] = [];
  const output: OutputMapJson[] = [];
  for (let k = 0; k < rank; k++) {
    const [lo, hi, map] = sliceDimension(toBig(start[k]), toBig(stop[k]), step[k], k);
    inclusiveMin.push(lo);
    exclusiveMax.push(hi);
    output.push(map);
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
      throw new NdselError(Reason.RankMismatch, "all points must have equal dimensionality");
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

/** The members of each `outer` axis-selector form (spec 5.5). */
const RANGE_FIELDS = new Set(["start", "stop", "step"]);

/**
 * Discriminate an object selector by the members present, then enforce its
 * closed member set: indices > intervals > range.
 */
function outerAxisForm(raw: Record<string, unknown>, k: number): "indices" | "intervals" | "range" {
  const [form, allowed] =
    "indices" in raw
      ? (["indices", new Set(["indices"])] as const)
      : "intervals" in raw
        ? (["intervals", new Set(["intervals"])] as const)
        : (["range", RANGE_FIELDS] as const);
  for (const key of Object.keys(raw)) {
    if (!allowed.has(key)) {
      throw new NdselError(Reason.UnknownField, `outer.axes[${k}]: unrecognized field for a ${form} selector: ${key}`);
    }
  }
  return form;
}

function expandIntervals(raw: unknown, k: number): Int[] {
  const coords: Int[] = [];
  requireArray(raw, `outer.axes[${k}].intervals`).forEach((pair, j) => {
    const what = `outer.axes[${k}].intervals[${j}]`;
    const bounds = requireIntArray(pair, what);
    if (bounds.length !== 2) throw new NdselError(Reason.InvalidJson, `${what} must be a [lo, hi] pair`);
    const [lo, hi] = bounds.map(toBig);
    if (lo > hi) throw new NdselError(Reason.BoundsOutOfOrder, `${what}: lo ${lo} exceeds hi ${hi}`);
    for (let c = lo; c < hi; c++) coords.push(demote(c));
  });
  return coords;
}

/** `coords` reshaped to the broadcast shape: `coords.length` in dimension k, 1 elsewhere. */
function broadcast(coords: Int[], rank: number, k: number): unknown {
  let nested: unknown = coords.map((c) => {
    let leaf: unknown = c;
    for (let d = k + 1; d < rank; d++) leaf = [leaf];
    return leaf;
  });
  for (let d = 0; d < k; d++) nested = [nested];
  return nested;
}

export function desugarOuter(msg: OuterMessage): Transform {
  const m = msg as { axes?: unknown; labels?: unknown };
  const axes = requireArray(m.axes, "outer.axes");
  const rank = axes.length;
  let labels = Array.from({ length: rank }, () => "");
  if (m.labels !== undefined) {
    labels = requireStringArray(m.labels, "outer.labels");
    if (labels.length !== rank) throw new NdselError(Reason.RankMismatch, "labels length must match axes");
  }

  const inclusiveMin: BoundJson[] = [];
  const exclusiveMax: BoundJson[] = [];
  const output: OutputMapJson[] = [];
  axes.forEach((raw, k) => {
    if (raw === null) {
      // The whole axis: implicit infinite bounds, resolved against the array.
      inclusiveMin.push(["-inf"]);
      exclusiveMax.push(["+inf"]);
      output.push({ offset: 0, stride: 1, input_dimension: k });
      return;
    }
    if (typeof raw !== "object" || Array.isArray(raw)) {
      throw new NdselError(Reason.InvalidJson, `outer.axes[${k}] must be null or an object, got ${JSON.stringify(raw)}`);
    }
    const sel = raw as Record<string, unknown>;
    const form = outerAxisForm(sel, k);
    if (form === "range") {
      if (!("start" in sel) || !("stop" in sel)) {
        throw new NdselError(Reason.InvalidJson, `outer.axes[${k}]: a range needs start and stop`);
      }
      const start = toBig(requireInt(sel.start, `outer.axes[${k}].start`));
      const stop = toBig(requireInt(sel.stop, `outer.axes[${k}].stop`));
      const step = "step" in sel ? toBig(requireInt(sel.step, `outer.axes[${k}].step`)) : 1n;
      const [lo, hi, map] = sliceDimension(start, stop, step, k);
      inclusiveMin.push(lo);
      exclusiveMax.push(hi);
      output.push(map);
      return;
    }
    const coords =
      form === "indices"
        ? requireIntArray(sel.indices, `outer.axes[${k}].indices`)
        : expandIntervals(sel.intervals, k);
    inclusiveMin.push(0);
    exclusiveMax.push(coords.length);
    // Empty and single-element lists collapse to constant maps (spec 7.1).
    if (coords.length === 0) output.push({ offset: 0 });
    else if (coords.length === 1) output.push({ offset: coords[0] });
    else {
      output.push({
        offset: 0,
        stride: 1,
        index_array: broadcast(coords, rank, k),
        index_array_bounds: ["-inf", "+inf"],
      });
    }
  });

  return {
    input_rank: rank,
    input_inclusive_min: inclusiveMin,
    input_exclusive_max: exclusiveMax,
    input_labels: labels,
    output,
  };
}
