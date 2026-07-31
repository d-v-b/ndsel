from ndsel.errors import NdselError, Reason


def test_reason_codes_are_stable():
    assert Reason.STEP_ZERO.value == "step_zero"
    assert Reason.MULTIPLE_UPPER_BOUNDS.value == "multiple_upper_bounds"
    assert Reason.BOUNDS_OUT_OF_ORDER.value == "bounds_out_of_order"
    assert Reason.RANK_MISMATCH.value == "rank_mismatch"
    assert Reason.UNKNOWN_KIND.value == "unknown_kind"
    assert Reason.INVALID_JSON.value == "invalid_json"


def test_error_carries_reason_and_detail():
    err = NdselError(Reason.STEP_ZERO, "step must be non-zero")
    assert err.reason is Reason.STEP_ZERO
    assert "step must be non-zero" in str(err)
