/** Stable machine-readable reason codes for rejected messages (the wire codes). */
export const Reason = {
  StepZero: "step_zero",
  MultipleUpperBounds: "multiple_upper_bounds",
  BoundsOutOfOrder: "bounds_out_of_order",
  OutputMapConflict: "output_map_conflict",
  RankMismatch: "rank_mismatch",
  UnknownKind: "unknown_kind",
  UnknownField: "unknown_field",
  InvalidJson: "invalid_json",
} as const;

export type ReasonCode = (typeof Reason)[keyof typeof Reason];

/** Raised when a message cannot be parsed or normalized. */
export class NdselError extends Error {
  reason: ReasonCode;
  detail: string;

  constructor(reason: ReasonCode, detail: string) {
    super(`${reason}: ${detail}`);
    this.name = "NdselError";
    this.reason = reason;
    this.detail = detail;
  }
}
