use serde::de::{self, Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};
use serde_json::Value;

/// A single index coordinate: a finite i64, or unbounded -inf / +inf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexValue {
    Finite(i64),
    NegInf,
    PosInf,
}

impl Serialize for IndexValue {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            IndexValue::Finite(n) => s.serialize_i64(*n),
            IndexValue::NegInf => s.serialize_str("-inf"),
            IndexValue::PosInf => s.serialize_str("+inf"),
        }
    }
}

impl<'de> Deserialize<'de> for IndexValue {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        match Value::deserialize(d)? {
            Value::Number(n) => n
                .as_i64()
                .map(IndexValue::Finite)
                .ok_or_else(|| de::Error::custom("index value must be an integer")),
            Value::String(s) if s == "-inf" => Ok(IndexValue::NegInf),
            Value::String(s) if s == "+inf" => Ok(IndexValue::PosInf),
            other => Err(de::Error::custom(format!("invalid index value: {other}"))),
        }
    }
}

#[cfg(test)]
mod index_value_tests {
    use super::*;

    fn round(json: &str) -> String {
        let v: IndexValue = serde_json::from_str(json).unwrap();
        serde_json::to_string(&v).unwrap()
    }

    #[test]
    fn finite_integer() {
        assert_eq!(round("7"), "7");
        assert!(matches!(serde_json::from_str::<IndexValue>("7").unwrap(), IndexValue::Finite(7)));
    }

    #[test]
    fn negative_infinity() {
        assert_eq!(round("\"-inf\""), "\"-inf\"");
        assert!(matches!(serde_json::from_str::<IndexValue>("\"-inf\"").unwrap(), IndexValue::NegInf));
    }

    #[test]
    fn positive_infinity() {
        assert_eq!(round("\"+inf\""), "\"+inf\"");
        assert!(matches!(serde_json::from_str::<IndexValue>("\"+inf\"").unwrap(), IndexValue::PosInf));
    }
}
