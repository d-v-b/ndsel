import type { BoundJson, Int } from "./values.ts";
import type { BoxMessage, PointMessage, PointsMessage, SliceMessage } from "./messages.ts";

export function point(coords: Int[]): PointMessage {
  return { kind: "point", coords };
}

export interface BoxOptions {
  inclusiveMin?: BoundJson[];
  exclusiveMax?: BoundJson[];
  inclusiveMax?: BoundJson[];
  shape?: BoundJson[];
  labels?: string[];
}

export function box(opts: BoxOptions = {}): BoxMessage {
  const msg: BoxMessage = { kind: "box" };
  if (opts.inclusiveMin !== undefined) msg.inclusive_min = opts.inclusiveMin;
  if (opts.exclusiveMax !== undefined) msg.exclusive_max = opts.exclusiveMax;
  if (opts.inclusiveMax !== undefined) msg.inclusive_max = opts.inclusiveMax;
  if (opts.shape !== undefined) msg.shape = opts.shape;
  if (opts.labels !== undefined) msg.labels = opts.labels;
  return msg;
}

export interface SliceOptions {
  start: Int[];
  stop: Int[];
  step?: Int[];
  labels?: string[];
}

export function slice(opts: SliceOptions): SliceMessage {
  const msg: SliceMessage = { kind: "slice", start: opts.start, stop: opts.stop };
  if (opts.step !== undefined) msg.step = opts.step;
  if (opts.labels !== undefined) msg.labels = opts.labels;
  return msg;
}

export function points(coords: Int[][]): PointsMessage {
  return { kind: "points", coords };
}
