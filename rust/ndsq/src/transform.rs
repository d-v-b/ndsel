use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde::Deserialize;

use crate::domain::{Domain, RawDomain};
use crate::error::NdsqError;
use crate::output::{OutputMap, RawOutputMap};
use crate::value::ImplicitValue;

/// The canonical core. Deserializes from the `transform` message body using the
/// `input_`-prefixed field names (distinct from the un-prefixed `box` names).
/// The `kind` field present in the message is ignored (no `deny_unknown_fields`).
#[derive(Debug, Clone, Deserialize)]
pub struct Transform {
    #[serde(rename = "input_rank", default)]
    rank: Option<usize>,
    #[serde(rename = "input_inclusive_min", default)]
    inclusive_min: Option<Vec<ImplicitValue>>,
    #[serde(rename = "input_exclusive_max", default)]
    exclusive_max: Option<Vec<ImplicitValue>>,
    #[serde(rename = "input_inclusive_max", default)]
    inclusive_max: Option<Vec<ImplicitValue>>,
    #[serde(rename = "input_shape", default)]
    shape: Option<Vec<ImplicitValue>>,
    #[serde(rename = "input_labels", default)]
    labels: Option<Vec<String>>,
    #[serde(default)]
    output: Option<Vec<RawOutputMap>>,

    // Populated by canonicalize(); never deserialized.
    #[serde(skip)]
    canon: Option<CanonTransform>,
}

#[derive(Debug, Clone)]
struct CanonTransform {
    domain: Domain,
    output: Vec<OutputMap>,
}

impl Transform {
    /// Produce the canonical form: domain normalized, output explicit (identity
    /// if omitted), all maps default-filled.
    pub fn canonicalize(self) -> Result<Transform, NdsqError> {
        // Reuse the shared domain canonicalization by mapping the input_-prefixed
        // fields onto a RawDomain.
        let raw_domain = RawDomain {
            rank: self.rank,
            inclusive_min: self.inclusive_min,
            exclusive_max: self.exclusive_max,
            inclusive_max: self.inclusive_max,
            shape: self.shape,
            labels: self.labels,
        };
        let domain = raw_domain.into_domain()?;
        let output = match self.output {
            Some(maps) => maps.into_iter().map(RawOutputMap::canonicalize).collect(),
            None => identity_output(&domain),
        };
        Ok(from_parts(domain, output))
    }
}

/// Identity: one single_input_dimension map per input dimension.
fn identity_output(domain: &Domain) -> Vec<OutputMap> {
    (0..domain.rank)
        .map(|k| OutputMap::SingleInputDimension { offset: 0, stride: 1, input_dimension: k })
        .collect()
}

impl Serialize for Transform {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // A Transform must be canonicalized before serialization.
        let canon = self
            .canon
            .as_ref()
            .expect("serialize a Transform only after canonicalize()");
        let mut st = s.serialize_struct("Transform", 5)?;
        st.serialize_field("input_rank", &canon.domain.rank)?;
        st.serialize_field("input_inclusive_min", &canon.domain.inclusive_min)?;
        st.serialize_field("input_exclusive_max", &canon.domain.exclusive_max)?;
        st.serialize_field("input_labels", &canon.domain.labels)?;
        st.serialize_field("output", &canon.output)?;
        st.end()
    }
}

/// Constructor used by the shorthand desugarers to build a canonical transform directly.
pub(crate) fn from_parts(domain: Domain, output: Vec<OutputMap>) -> Transform {
    Transform {
        rank: None,
        inclusive_min: None,
        exclusive_max: None,
        inclusive_max: None,
        shape: None,
        labels: None,
        output: None,
        canon: Some(CanonTransform { domain, output }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omitted_output_becomes_explicit_identity() {
        // 2-D box-like domain, no output -> identity single_input_dimension maps.
        let raw: Transform = serde_json::from_str(
            r#"{ "input_inclusive_min": [0, 0], "input_exclusive_max": [3, 4] }"#,
        ).unwrap();
        let canon = raw.canonicalize().unwrap();
        let v = serde_json::to_value(&canon).unwrap();
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
    fn canonicalize_is_idempotent() {
        let raw: Transform = serde_json::from_str(
            r#"{ "input_inclusive_min": [0], "input_shape": [5] }"#,
        ).unwrap();
        let once = raw.canonicalize().unwrap();
        let twice: Transform =
            serde_json::from_value(serde_json::to_value(&once).unwrap()).unwrap();
        let twice = twice.canonicalize().unwrap();
        assert_eq!(
            serde_json::to_value(&once).unwrap(),
            serde_json::to_value(&twice).unwrap()
        );
    }
}
