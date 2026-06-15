use serde::Deserialize;

use crate::domain::{Domain, RawDomain};
use crate::error::NdselError;
use crate::output::OutputMap;
use crate::transform::{from_parts, Transform};
use crate::value::{ImplicitValue, IndexValue};

/// `{ "kind": "point", "coords": [...] }`
#[derive(Debug, Clone, Deserialize)]
pub struct Point {
    pub coords: Vec<i64>,
}

impl Point {
    pub fn desugar(self) -> Result<Transform, NdselError> {
        let domain = Domain {
            rank: 0,
            inclusive_min: vec![],
            exclusive_max: vec![],
            labels: vec![],
        };
        let output = self
            .coords
            .into_iter()
            .map(|c| OutputMap::Constant { offset: c })
            .collect();
        Ok(from_parts(domain, output))
    }
}

/// `{ "kind": "box", inclusive_min/exclusive_max/inclusive_max/shape, labels? }`
#[derive(Debug, Clone, Deserialize)]
pub struct BoxSel {
    #[serde(flatten)]
    domain: RawDomain,
}

impl BoxSel {
    pub fn desugar(self) -> Result<Transform, NdselError> {
        let domain = self.domain.into_domain()?;
        let output = (0..domain.rank)
            .map(|k| OutputMap::SingleInputDimension { offset: 0, stride: 1, input_dimension: k })
            .collect();
        Ok(from_parts(domain, output))
    }
}

/// `{ "kind": "slice", "start": [...], "stop": [...], "step": [...]?, "labels": [...]? }`
#[derive(Debug, Clone, Deserialize)]
pub struct Slice {
    pub start: Vec<i64>,
    pub stop: Vec<i64>,
    #[serde(default)]
    pub step: Option<Vec<i64>>,
    #[serde(default)]
    pub labels: Option<Vec<String>>,
}

/// `{ "kind": "points", "coords": [[...], [...], ...] }`
#[derive(Debug, Clone, Deserialize)]
pub struct Points {
    pub coords: Vec<Vec<i64>>,
}

impl Slice {
    pub fn desugar(self) -> Result<Transform, NdselError> {
        let rank = self.start.len();
        if self.stop.len() != rank {
            return Err(NdselError::new(
                crate::error::Reason::RankMismatch,
                "start and stop must have equal length",
            ));
        }
        let step: Vec<i64> = match self.step {
            Some(s) => {
                if s.len() != rank {
                    return Err(NdselError::new(
                        crate::error::Reason::RankMismatch,
                        "step length must match start/stop",
                    ));
                }
                s
            }
            None => vec![1; rank],
        };
        if let Some(labels) = &self.labels {
            if labels.len() != rank {
                return Err(NdselError::new(
                    crate::error::Reason::RankMismatch,
                    "labels length must match start/stop",
                ));
            }
        }

        let mut inclusive_min = Vec::with_capacity(rank);
        let mut exclusive_max = Vec::with_capacity(rank);
        let mut output = Vec::with_capacity(rank);

        for k in 0..rank {
            let (a, b, s) = (self.start[k], self.stop[k], step[k]);
            if s == 0 {
                return Err(NdselError::new(crate::error::Reason::StepZero, "step must be non-zero"));
            }
            if s < 0 {
                return Err(NdselError::new(
                    crate::error::Reason::NegativeStepUnsupported,
                    "negative step is not yet specified",
                ));
            }
            // s > 0
            let m = if b <= a { 0 } else { ceil_div(b - a, s) };
            let o = a.div_euclid(s); // floor(a/s) for s > 0
            let offset = a - s * o; // == a.rem_euclid(s), the lattice phase in [0, s)
            inclusive_min.push(fin(o));
            exclusive_max.push(fin(o + m));
            output.push(OutputMap::SingleInputDimension {
                offset,
                stride: s,
                input_dimension: k,
            });
        }

        let labels = self.labels.unwrap_or_else(|| vec![String::new(); rank]);
        let domain = Domain { rank, inclusive_min, exclusive_max, labels };
        Ok(from_parts(domain, output))
    }
}

/// Ceiling of p/q for p >= 0, q > 0.
fn ceil_div(p: i64, q: i64) -> i64 {
    (p + q - 1) / q
}

impl Points {
    pub fn desugar(self) -> Result<Transform, NdselError> {
        let m = self.coords.len();
        // Output rank = point dimensionality (0 if there are no points).
        let n = self.coords.first().map(|p| p.len()).unwrap_or(0);
        for p in &self.coords {
            if p.len() != n {
                return Err(NdselError::new(
                    crate::error::Reason::RankMismatch,
                    "all points must have equal dimensionality",
                ));
            }
        }

        // input domain: single dimension [0, m)
        let domain = Domain {
            rank: 1,
            inclusive_min: vec![fin(0)],
            exclusive_max: vec![fin(m as i64)],
            labels: vec![String::new()],
        };

        // one index_array map per output dimension k: column k across all points
        let mut output = Vec::with_capacity(n);
        for k in 0..n {
            let column: Vec<serde_json::Value> = self
                .coords
                .iter()
                .map(|p| serde_json::Value::from(p[k]))
                .collect();
            output.push(OutputMap::IndexArray {
                offset: 0,
                stride: 1,
                index_array: serde_json::Value::Array(column),
                bounds: (IndexValue::NegInf, IndexValue::PosInf),
            });
        }

        Ok(from_parts(domain, output))
    }
}

/// An explicit finite bound, used by the slice/points desugarers.
fn fin(n: i64) -> ImplicitValue { ImplicitValue::explicit(IndexValue::Finite(n)) }

#[cfg(test)]
mod slice_tests {

    fn norm(json: &str) -> serde_json::Value {
        let msg = crate::parse(json).unwrap();
        serde_json::to_value(crate::normalize(msg).unwrap()).unwrap()
    }
    fn err(json: &str) -> crate::Reason {
        let msg = crate::parse(json).unwrap();
        crate::normalize(msg).unwrap_err().reason
    }

    #[test]
    fn unit_step_preserves_source_frame() {
        // x[5:10] -> domain [5,10), identity map (coordinate-preserving)
        let v = norm(r#"{ "kind": "slice", "start": [5], "stop": [10] }"#);
        assert_eq!(v["input_inclusive_min"], serde_json::json!([5]));
        assert_eq!(v["input_exclusive_max"], serde_json::json!([10]));
        assert_eq!(
            v["output"],
            serde_json::json!([{ "offset": 0, "stride": 1, "input_dimension": 0 }])
        );
    }

    #[test]
    fn divisible_strided_slice() {
        // x[4:10:2] selects {4,6,8}; o=2, offset=0, domain [2,5)
        let v = norm(r#"{ "kind": "slice", "start": [4], "stop": [10], "step": [2] }"#);
        assert_eq!(v["input_inclusive_min"], serde_json::json!([2]));
        assert_eq!(v["input_exclusive_max"], serde_json::json!([5]));
        assert_eq!(
            v["output"],
            serde_json::json!([{ "offset": 0, "stride": 2, "input_dimension": 0 }])
        );
    }

    #[test]
    fn nondivisible_strided_slice_uses_phase_offset() {
        // x[5:10:2] selects {5,7,9}; o=floor(5/2)=2, offset=5-2*2=1, domain [2,5)
        let v = norm(r#"{ "kind": "slice", "start": [5], "stop": [10], "step": [2] }"#);
        assert_eq!(v["input_inclusive_min"], serde_json::json!([2]));
        assert_eq!(v["input_exclusive_max"], serde_json::json!([5]));
        assert_eq!(
            v["output"],
            serde_json::json!([{ "offset": 1, "stride": 2, "input_dimension": 0 }])
        );
    }

    #[test]
    fn empty_slice_has_zero_length_dim() {
        // x[10:10] -> m=0, o=10, domain [10,10)
        let v = norm(r#"{ "kind": "slice", "start": [10], "stop": [10] }"#);
        assert_eq!(v["input_inclusive_min"], serde_json::json!([10]));
        assert_eq!(v["input_exclusive_max"], serde_json::json!([10]));
    }

    #[test]
    fn step_zero_is_error() {
        assert_eq!(err(r#"{ "kind": "slice", "start": [0], "stop": [4], "step": [0] }"#), crate::Reason::StepZero);
    }

    #[test]
    fn negative_step_is_unsupported() {
        assert_eq!(
            err(r#"{ "kind": "slice", "start": [9], "stop": [-1], "step": [-2] }"#),
            crate::Reason::NegativeStepUnsupported
        );
    }

    #[test]
    fn slice_length_mismatch_is_rank_mismatch() {
        assert_eq!(
            err(r#"{ "kind": "slice", "start": [0, 0], "stop": [4] }"#),
            crate::Reason::RankMismatch
        );
    }
}

#[cfg(test)]
mod points_tests {

    fn norm(json: &str) -> serde_json::Value {
        let msg = crate::parse(json).unwrap();
        serde_json::to_value(crate::normalize(msg).unwrap()).unwrap()
    }

    #[test]
    fn points_transpose_to_columnar_index_arrays() {
        // three 2-D points: (1,10), (2,20), (3,30)
        let v = norm(r#"{ "kind": "points", "coords": [[1, 10], [2, 20], [3, 30]] }"#);
        assert_eq!(v["input_rank"], 1);
        assert_eq!(v["input_inclusive_min"], serde_json::json!([0]));
        assert_eq!(v["input_exclusive_max"], serde_json::json!([3]));
        assert_eq!(
            v["output"],
            serde_json::json!([
                { "offset": 0, "stride": 1, "index_array": [1, 2, 3],   "index_array_bounds": ["-inf", "+inf"] },
                { "offset": 0, "stride": 1, "index_array": [10, 20, 30], "index_array_bounds": ["-inf", "+inf"] }
            ])
        );
    }

    #[test]
    fn empty_points_is_zero_length_with_unknown_rank_zero() {
        // No points -> m=0, output rank 0 (no columns to emit)
        let v = norm(r#"{ "kind": "points", "coords": [] }"#);
        assert_eq!(v["input_rank"], 1);
        assert_eq!(v["input_exclusive_max"], serde_json::json!([0]));
        assert_eq!(v["output"], serde_json::json!([]));
    }

    #[test]
    fn ragged_points_is_rank_mismatch() {
        let msg = crate::parse(r#"{ "kind": "points", "coords": [[1, 2], [3]] }"#).unwrap();
        assert_eq!(crate::normalize(msg).unwrap_err().reason, crate::Reason::RankMismatch);
    }
}

#[cfg(test)]
mod point_box_tests {

    fn norm(json: &str) -> serde_json::Value {
        let msg = crate::parse(json).unwrap();
        serde_json::to_value(crate::normalize(msg).unwrap()).unwrap()
    }

    #[test]
    fn point_desugars_to_constant_maps() {
        let v = norm(r#"{ "kind": "point", "coords": [4, 7] }"#);
        assert_eq!(v["input_rank"], 0);
        assert_eq!(v["input_inclusive_min"], serde_json::json!([]));
        assert_eq!(v["input_exclusive_max"], serde_json::json!([]));
        assert_eq!(
            v["output"],
            serde_json::json!([{ "offset": 4 }, { "offset": 7 }])
        );
    }

    #[test]
    fn box_desugars_to_identity() {
        let v = norm(r#"{ "kind": "box", "inclusive_min": [0, 0], "exclusive_max": [3, 4] }"#);
        assert_eq!(v["input_inclusive_min"], serde_json::json!([0, 0]));
        assert_eq!(v["input_exclusive_max"], serde_json::json!([3, 4]));
        assert_eq!(
            v["output"],
            serde_json::json!([
                { "offset": 0, "stride": 1, "input_dimension": 0 },
                { "offset": 0, "stride": 1, "input_dimension": 1 }
            ])
        );
    }

    #[test]
    fn box_shape_only_defaults_origin_zero() {
        let v = norm(r#"{ "kind": "box", "shape": [5] }"#);
        assert_eq!(v["input_inclusive_min"], serde_json::json!([0]));
        assert_eq!(v["input_exclusive_max"], serde_json::json!([5]));
    }
}
