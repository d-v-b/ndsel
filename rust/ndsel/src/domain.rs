use serde::{Deserialize, Serialize};

use crate::error::{NdselError, Reason};
use crate::value::{ImplicitValue, IndexValue};

/// The canonical input domain: per-dimension [inclusive_min, exclusive_max) + labels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Domain {
    pub rank: usize,
    pub inclusive_min: Vec<ImplicitValue>,
    pub exclusive_max: Vec<ImplicitValue>,
    pub labels: Vec<String>,
}

/// As-received domain fields, before canonicalization.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RawDomain {
    #[serde(default)]
    pub rank: Option<usize>,
    #[serde(default)]
    pub inclusive_min: Option<Vec<ImplicitValue>>,
    #[serde(default)]
    pub exclusive_max: Option<Vec<ImplicitValue>>,
    #[serde(default)]
    pub inclusive_max: Option<Vec<ImplicitValue>>,
    #[serde(default)]
    pub shape: Option<Vec<ImplicitValue>>,
    #[serde(default)]
    pub labels: Option<Vec<String>>,
}

impl RawDomain {
    pub fn into_domain(self) -> Result<Domain, NdselError> {
        // Exactly one upper-bound spelling.
        let upper_count = self.exclusive_max.is_some() as u8
            + self.inclusive_max.is_some() as u8
            + self.shape.is_some() as u8;
        if upper_count > 1 {
            return Err(NdselError::new(
                Reason::MultipleUpperBounds,
                "specify only one of exclusive_max / inclusive_max / shape",
            ));
        }

        // Determine rank from whichever arrays are present.
        let mut rank: Option<usize> = self.rank;
        let check = |len: usize, rank: &mut Option<usize>| -> Result<(), NdselError> {
            match rank {
                Some(r) if *r != len => Err(NdselError::new(
                    Reason::RankMismatch,
                    format!("array length {len} disagrees with rank {r}"),
                )),
                _ => {
                    *rank = Some(len);
                    Ok(())
                }
            }
        };
        if let Some(v) = &self.inclusive_min { check(v.len(), &mut rank)?; }
        if let Some(v) = &self.exclusive_max { check(v.len(), &mut rank)?; }
        if let Some(v) = &self.inclusive_max { check(v.len(), &mut rank)?; }
        if let Some(v) = &self.shape { check(v.len(), &mut rank)?; }
        if let Some(v) = &self.labels { check(v.len(), &mut rank)?; }
        let rank = rank.unwrap_or(0);

        // inclusive_min: default to explicit 0 per dimension.
        let inclusive_min = self.inclusive_min.unwrap_or_else(|| {
            vec![ImplicitValue::explicit(IndexValue::Finite(0)); rank]
        });

        // Canonicalize the upper bound to exclusive_max.
        let exclusive_max = if let Some(em) = self.exclusive_max {
            em
        } else if let Some(im) = self.inclusive_max {
            im.into_iter().map(bump_inclusive_to_exclusive).collect()
        } else if let Some(shape) = self.shape {
            inclusive_min
                .iter()
                .zip(shape.iter())
                .map(|(lo, sz)| add_shape(*lo, *sz))
                .collect()
        } else {
            // No upper bound given: unbounded implicit +inf.
            vec![ImplicitValue { value: IndexValue::PosInf, implicit: true }; rank]
        };

        for (lo, hi) in inclusive_min.iter().zip(exclusive_max.iter()) {
            if !min_le_max(lo.value, hi.value) {
                return Err(NdselError::new(
                    Reason::BoundsOutOfOrder,
                    "inclusive_min must not exceed exclusive_max",
                ));
            }
        }

        let labels = self.labels.unwrap_or_else(|| vec![String::new(); rank]);

        Ok(Domain { rank, inclusive_min, exclusive_max, labels })
    }
}

/// Extended-integer order: `-inf <= x <= +inf`; finite values compared numerically.
fn min_le_max(min: IndexValue, max: IndexValue) -> bool {
    use IndexValue::{Finite, NegInf, PosInf};
    match (min, max) {
        (NegInf, _) | (_, PosInf) => true,
        (PosInf, _) | (_, NegInf) => false,
        (Finite(a), Finite(b)) => a <= b,
    }
}

/// inclusive_max n -> exclusive_max n+1 (preserving implicit flag; +inf stays +inf).
fn bump_inclusive_to_exclusive(v: ImplicitValue) -> ImplicitValue {
    let value = match v.value {
        IndexValue::Finite(n) => IndexValue::Finite(n + 1),
        other => other,
    };
    ImplicitValue { value, implicit: v.implicit }
}

/// exclusive_max = inclusive_min + shape (preserving min's implicit flag; inf saturates).
fn add_shape(lo: ImplicitValue, sz: ImplicitValue) -> ImplicitValue {
    let value = match (lo.value, sz.value) {
        (IndexValue::Finite(a), IndexValue::Finite(s)) => IndexValue::Finite(a + s),
        (IndexValue::PosInf, _) | (_, IndexValue::PosInf) => IndexValue::PosInf,
        (IndexValue::NegInf, _) | (_, IndexValue::NegInf) => IndexValue::NegInf,
    };
    ImplicitValue { value, implicit: lo.implicit }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::{ImplicitValue, IndexValue};

    fn fin(n: i64) -> ImplicitValue { ImplicitValue::explicit(IndexValue::Finite(n)) }

    #[test]
    fn shape_only_defaults_min_to_zero() {
        let raw: RawDomain = serde_json::from_str(r#"{ "shape": [3, 4] }"#).unwrap();
        let d = raw.into_domain().unwrap();
        assert_eq!(d.rank, 2);
        assert_eq!(d.inclusive_min, vec![fin(0), fin(0)]);
        assert_eq!(d.exclusive_max, vec![fin(3), fin(4)]);
    }

    #[test]
    fn inclusive_max_is_converted_to_exclusive() {
        let raw: RawDomain =
            serde_json::from_str(r#"{ "inclusive_min": [0], "inclusive_max": [9] }"#).unwrap();
        let d = raw.into_domain().unwrap();
        assert_eq!(d.exclusive_max, vec![fin(10)]);
    }

    #[test]
    fn multiple_upper_bounds_is_error() {
        let raw: RawDomain =
            serde_json::from_str(r#"{ "shape": [3], "exclusive_max": [3] }"#).unwrap();
        assert_eq!(raw.into_domain().unwrap_err().reason, crate::error::Reason::MultipleUpperBounds);
    }

    #[test]
    fn length_disagreement_is_rank_mismatch() {
        let raw: RawDomain =
            serde_json::from_str(r#"{ "inclusive_min": [0, 0], "shape": [3] }"#).unwrap();
        assert_eq!(raw.into_domain().unwrap_err().reason, crate::error::Reason::RankMismatch);
    }
}
