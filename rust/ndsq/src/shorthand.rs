use serde::Deserialize;

use crate::domain::{Domain, RawDomain};
use crate::error::NdsqError;
use crate::output::OutputMap;
use crate::transform::{from_parts, Transform};
use crate::value::{ImplicitValue, IndexValue};

/// `{ "kind": "point", "coords": [...] }`
#[derive(Debug, Clone, Deserialize)]
pub struct Point {
    pub coords: Vec<i64>,
}

impl Point {
    pub fn desugar(self) -> Result<Transform, NdsqError> {
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
    pub fn desugar(self) -> Result<Transform, NdsqError> {
        let domain = self.domain.into_domain()?;
        let output = (0..domain.rank)
            .map(|k| OutputMap::SingleInputDimension { offset: 0, stride: 1, input_dimension: k })
            .collect();
        Ok(from_parts(domain, output))
    }
}

// Placeholders filled in Tasks 10–11 so `lib.rs` compiles. Replace these in those tasks.
#[derive(Debug, Clone, Deserialize)]
pub struct Slice {
    pub start: Vec<i64>,
    pub stop: Vec<i64>,
    #[serde(default)]
    pub step: Option<Vec<i64>>,
    #[serde(default)]
    pub labels: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Points {
    pub coords: Vec<Vec<i64>>,
}

impl Slice {
    pub fn desugar(self) -> Result<Transform, NdsqError> {
        // Implemented in Task 10.
        let _ = (&self.start, &self.stop, &self.step, &self.labels);
        unimplemented!("Slice::desugar — Task 10")
    }
}

impl Points {
    pub fn desugar(self) -> Result<Transform, NdsqError> {
        // Implemented in Task 11.
        let _ = &self.coords;
        unimplemented!("Points::desugar — Task 11")
    }
}

// Used by slice/points desugaring helpers.
#[allow(dead_code)]
fn fin(n: i64) -> ImplicitValue { ImplicitValue::explicit(IndexValue::Finite(n)) }

#[cfg(test)]
mod point_box_tests {
    use super::*;

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
