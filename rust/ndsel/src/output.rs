use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde::Deserialize;
use serde_json::Value;

use crate::value::IndexValue;

/// One output index map (canonical form).
#[derive(Debug, Clone, PartialEq)]
pub enum OutputMap {
    Constant { offset: i64 },
    SingleInputDimension { offset: i64, stride: i64, input_dimension: usize },
    IndexArray { offset: i64, stride: i64, index_array: Value, bounds: (IndexValue, IndexValue) },
}

/// As-received output map before default filling / kind discrimination.
#[derive(Debug, Clone, Deserialize)]
pub struct RawOutputMap {
    #[serde(default)]
    pub offset: Option<i64>,
    #[serde(default)]
    pub stride: Option<i64>,
    #[serde(default)]
    pub input_dimension: Option<usize>,
    #[serde(default)]
    pub index_array: Option<Value>,
    #[serde(default)]
    pub index_array_bounds: Option<(IndexValue, IndexValue)>,
}

impl RawOutputMap {
    pub fn canonicalize(self) -> OutputMap {
        let offset = self.offset.unwrap_or(0);
        let stride = self.stride.unwrap_or(1);
        if let Some(arr) = self.index_array {
            let bounds = self
                .index_array_bounds
                .unwrap_or((IndexValue::NegInf, IndexValue::PosInf));
            OutputMap::IndexArray { offset, stride, index_array: arr, bounds }
        } else if let Some(dim) = self.input_dimension {
            OutputMap::SingleInputDimension { offset, stride, input_dimension: dim }
        } else {
            OutputMap::Constant { offset }
        }
    }
}

impl Serialize for OutputMap {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            OutputMap::Constant { offset } => {
                let mut st = s.serialize_struct("OutputMap", 1)?;
                st.serialize_field("offset", offset)?;
                st.end()
            }
            OutputMap::SingleInputDimension { offset, stride, input_dimension } => {
                let mut st = s.serialize_struct("OutputMap", 3)?;
                st.serialize_field("offset", offset)?;
                st.serialize_field("stride", stride)?;
                st.serialize_field("input_dimension", input_dimension)?;
                st.end()
            }
            OutputMap::IndexArray { offset, stride, index_array, bounds } => {
                let mut st = s.serialize_struct("OutputMap", 4)?;
                st.serialize_field("offset", offset)?;
                st.serialize_field("stride", stride)?;
                st.serialize_field("index_array", index_array)?;
                st.serialize_field("index_array_bounds", &[bounds.0, bounds.1])?;
                st.end()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_map_carries_only_offset() {
        let m: RawOutputMap = serde_json::from_str(r#"{ "offset": 3 }"#).unwrap();
        let canon = m.canonicalize();
        assert_eq!(canon, OutputMap::Constant { offset: 3 });
        assert_eq!(serde_json::to_value(&canon).unwrap(), serde_json::json!({ "offset": 3 }));
    }

    #[test]
    fn single_input_dimension_fills_defaults() {
        let m: RawOutputMap = serde_json::from_str(r#"{ "input_dimension": 2 }"#).unwrap();
        let canon = m.canonicalize();
        assert_eq!(
            serde_json::to_value(&canon).unwrap(),
            serde_json::json!({ "offset": 0, "stride": 1, "input_dimension": 2 })
        );
    }

    #[test]
    fn index_array_fills_offset_stride_and_bounds() {
        let m: RawOutputMap =
            serde_json::from_str(r#"{ "index_array": [1, 2, 3] }"#).unwrap();
        let canon = m.canonicalize();
        assert_eq!(
            serde_json::to_value(&canon).unwrap(),
            serde_json::json!({
                "offset": 0, "stride": 1,
                "index_array": [1, 2, 3],
                "index_array_bounds": ["-inf", "+inf"]
            })
        );
    }
}
