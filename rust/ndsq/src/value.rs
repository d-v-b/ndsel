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

/// An index bound with an explicit/implicit flag.
/// JSON: a bare value is explicit; the same value wrapped in a 1-element
/// array is implicit (`7` vs `[7]`, `"-inf"` vs `["-inf"]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImplicitValue {
    pub value: IndexValue,
    pub implicit: bool,
}

impl ImplicitValue {
    pub fn explicit(value: IndexValue) -> Self {
        ImplicitValue { value, implicit: false }
    }
}

impl Serialize for ImplicitValue {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.implicit {
            // serialize as a 1-element array
            use serde::ser::SerializeSeq;
            let mut seq = s.serialize_seq(Some(1))?;
            seq.serialize_element(&self.value)?;
            seq.end()
        } else {
            self.value.serialize(s)
        }
    }
}

impl<'de> Deserialize<'de> for ImplicitValue {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        match v {
            Value::Array(items) => {
                if items.len() != 1 {
                    return Err(de::Error::custom("implicit bound must be a 1-element array"));
                }
                let inner: IndexValue = serde_json::from_value(items.into_iter().next().unwrap())
                    .map_err(de::Error::custom)?;
                Ok(ImplicitValue { value: inner, implicit: true })
            }
            other => {
                let inner: IndexValue = serde_json::from_value(other).map_err(de::Error::custom)?;
                Ok(ImplicitValue { value: inner, implicit: false })
            }
        }
    }
}

#[cfg(test)]
mod implicit_value_tests {
    use super::*;

    fn parse(json: &str) -> ImplicitValue {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn bare_value_is_explicit() {
        let v = parse("7");
        assert_eq!(v.value, IndexValue::Finite(7));
        assert!(!v.implicit);
    }

    #[test]
    fn bracketed_value_is_implicit() {
        let v = parse("[7]");
        assert_eq!(v.value, IndexValue::Finite(7));
        assert!(v.implicit);
    }

    #[test]
    fn bracketed_inf_is_implicit() {
        let v = parse("[\"-inf\"]");
        assert_eq!(v.value, IndexValue::NegInf);
        assert!(v.implicit);
    }

    #[test]
    fn explicit_serializes_bare_implicit_serializes_bracketed() {
        assert_eq!(serde_json::to_string(&parse("7")).unwrap(), "7");
        assert_eq!(serde_json::to_string(&parse("[7]")).unwrap(), "[7]");
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
