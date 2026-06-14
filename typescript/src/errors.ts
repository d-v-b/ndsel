/** Stable machine-readable reason codes for rejected messages (the wire codes). */
export const Reason = {
  StepZero: "step_zero",
  NegativeStepUnsupported: "negative_step_unsupported",
  MultipleUpperBounds: "multiple_upper_bounds",
  RankMismatch: "rank_mismatch",
  UnknownKind: "unknown_kind",
  InvalidJson: "invalid_json",
} as const;

export type ReasonCode = (typeof Reason)[keyof typeof Reason];

/** Raised when a message cannot be parsed or normalized. */
export class NdsqError extends Error {
  reason: ReasonCode;
  detail: string;

  constructor(reason: ReasonCode, detail: string) {
    super(`${reason}: ${detail}`);
    this.name = "NdsqError";
    this.reason = reason;
    this.detail = detail;
  }
}
