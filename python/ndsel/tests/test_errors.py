from ndsel.errors import NdsqError, Reason


def test_reason_codes_are_stable():
    assert Reason.STEP_ZERO.value == "step_zero"
    assert Reason.NEGATIVE_STEP_UNSUPPORTED.value == "negative_step_unsupported"
    assert Reason.MULTIPLE_UPPER_BOUNDS.value == "multiple_upper_bounds"
    assert Reason.RANK_MISMATCH.value == "rank_mismatch"
    assert Reason.UNKNOWN_KIND.value == "unknown_kind"
    assert Reason.INVALID_JSON.value == "invalid_json"


def test_error_carries_reason_and_detail():
    err = NdsqError(Reason.STEP_ZERO, "step must be non-zero")
    assert err.reason is Reason.STEP_ZERO
    assert "step must be non-zero" in str(err)
