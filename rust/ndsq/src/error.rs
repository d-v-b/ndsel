use std::fmt;

/// Stable machine-readable reason codes for rejected messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    StepZero,
    NegativeStepUnsupported,
    MultipleUpperBounds,
    RankMismatch,
    UnknownKind,
    InvalidJson,
}

impl Reason {
    /// The stable string used in corpus error fixtures.
    pub fn code(self) -> &'static str {
        match self {
            Reason::StepZero => "step_zero",
            Reason::NegativeStepUnsupported => "negative_step_unsupported",
            Reason::MultipleUpperBounds => "multiple_upper_bounds",
            Reason::RankMismatch => "rank_mismatch",
            Reason::UnknownKind => "unknown_kind",
            Reason::InvalidJson => "invalid_json",
        }
    }
}

/// An error produced while parsing or normalizing a message.
#[derive(Debug, Clone)]
pub struct NdsqError {
    pub reason: Reason,
    pub detail: String,
}

impl NdsqError {
    pub fn new(reason: Reason, detail: impl Into<String>) -> Self {
        NdsqError { reason, detail: detail.into() }
    }

    pub fn from_serde(err: serde_json::Error) -> Self {
        // serde's untagged/`kind` failures and type errors all collapse here.
        NdsqError::new(Reason::InvalidJson, err.to_string())
    }
}

impl fmt::Display for NdsqError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.reason.code(), self.detail)
    }
}

impl std::error::Error for NdsqError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reason_code_strings_are_stable() {
        assert_eq!(Reason::StepZero.code(), "step_zero");
        assert_eq!(Reason::RankMismatch.code(), "rank_mismatch");
        assert_eq!(Reason::MultipleUpperBounds.code(), "multiple_upper_bounds");
        assert_eq!(Reason::NegativeStepUnsupported.code(), "negative_step_unsupported");
        assert_eq!(Reason::UnknownKind.code(), "unknown_kind");
        assert_eq!(Reason::InvalidJson.code(), "invalid_json");
    }
}
