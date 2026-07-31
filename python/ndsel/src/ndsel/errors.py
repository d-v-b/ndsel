"""Stable error codes and the exception type for ndsel."""

from __future__ import annotations

from enum import Enum


class Reason(str, Enum):
    """Machine-readable reason codes for rejected messages (the wire codes)."""

    STEP_ZERO = "step_zero"
    MULTIPLE_UPPER_BOUNDS = "multiple_upper_bounds"
    BOUNDS_OUT_OF_ORDER = "bounds_out_of_order"
    OUTPUT_MAP_CONFLICT = "output_map_conflict"
    RANK_MISMATCH = "rank_mismatch"
    UNKNOWN_KIND = "unknown_kind"
    UNKNOWN_FIELD = "unknown_field"
    INVALID_JSON = "invalid_json"


class NdselError(Exception):
    """Raised when a message cannot be parsed or normalized."""

    def __init__(self, reason: Reason, detail: str) -> None:
        super().__init__(f"{reason.value}: {detail}")
        self.reason = reason
        self.detail = detail
