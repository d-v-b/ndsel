import type { BoundJson, IndexValue, Int } from "./values.ts";

export interface PointMessage {
  kind: "point";
  coords: Int[];
}

export interface BoxMessage {
  kind: "box";
  inclusive_min?: BoundJson[];
  exclusive_max?: BoundJson[];
  inclusive_max?: BoundJson[];
  shape?: BoundJson[];
  labels?: string[];
}

export interface SliceMessage {
  kind: "slice";
  start: Int[];
  stop: Int[];
  step?: Int[];
  labels?: string[];
}

export interface PointsMessage {
  kind: "points";
  coords: Int[][];
}

/** One `outer` axis selector; `null` selects the whole axis (spec 5.5). */
export type OuterAxis = { start: Int; stop: Int; step?: Int } | { indices: Int[] } | { intervals: [Int, Int][] } | null;

export interface OuterMessage {
  kind: "outer";
  axes: OuterAxis[];
  labels?: string[];
}

/** An output-map object as accepted on input (before default-filling). */
export interface OutputMapInput {
  offset?: Int;
  stride?: Int;
  input_dimension?: Int;
  index_array?: unknown;
  index_array_bounds?: [IndexValue, IndexValue];
}

export interface TransformMessage {
  kind: "transform";
  input_rank?: Int;
  input_inclusive_min?: BoundJson[];
  input_exclusive_max?: BoundJson[];
  input_inclusive_max?: BoundJson[];
  input_shape?: BoundJson[];
  input_labels?: string[];
  output?: OutputMapInput[];
}

/** Any ndsel message, discriminated on `kind`. */
export type Message = PointMessage | BoxMessage | SliceMessage | PointsMessage | OuterMessage | TransformMessage;
