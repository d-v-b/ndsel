//! ndsel — JSON-serializable n-dimensional spatial queries.
//!
//! A `Message` is a `kind`-discriminated union; `normalize` reduces any message
//! to a canonical `Transform`. Adapts the index model of Google tensorstore.
//!
//! Parse a JSON string with [`parse`], then reduce it to a canonical
//! [`Transform`] with [`normalize`].

mod domain;
mod error;
mod output;
mod shorthand;
mod transform;
mod value;

pub use domain::Domain;
pub use error::{NdselError, Reason};
pub use output::OutputMap;
pub use shorthand::{BoxSel, Point, Points, Slice};
pub use transform::Transform;
pub use value::{ImplicitValue, IndexValue};

/// A spatial-query message, discriminated by its `kind`.
///
/// Deliberately NOT a `#[serde(tag = "kind")]` enum: internally-tagged enums
/// combine poorly with `#[serde(flatten)]` (used by `BoxSel`). `parse` peeks
/// `kind` and dispatches manually, which is robust and yields a clean
/// `unknown_kind` error.
#[derive(Debug, Clone)]
pub enum Message {
    Point(shorthand::Point),
    Box(shorthand::BoxSel),
    Slice(shorthand::Slice),
    Points(shorthand::Points),
    Transform(Transform),
}

impl From<Point> for Message {
    fn from(p: Point) -> Self {
        Message::Point(p)
    }
}
impl From<BoxSel> for Message {
    fn from(b: BoxSel) -> Self {
        Message::Box(b)
    }
}
impl From<Slice> for Message {
    fn from(s: Slice) -> Self {
        Message::Slice(s)
    }
}
impl From<Points> for Message {
    fn from(p: Points) -> Self {
        Message::Points(p)
    }
}
impl From<Transform> for Message {
    fn from(t: Transform) -> Self {
        Message::Transform(t)
    }
}

/// Reduce any message to its canonical `Transform`.
pub fn normalize(message: Message) -> Result<Transform, NdselError> {
    match message {
        Message::Point(p) => p.desugar(),
        Message::Box(b) => b.desugar(),
        Message::Slice(s) => s.desugar(),
        Message::Points(p) => p.desugar(),
        Message::Transform(t) => t.canonicalize(),
    }
}

/// Parse a JSON string into a `Message`, dispatching on the `kind` discriminator.
/// Unknown `kind` values yield `unknown_kind`; a missing `kind` yields `invalid_json`.
pub fn parse(json: &str) -> Result<Message, NdselError> {
    use serde_json::{from_value, Value};
    let value: Value = serde_json::from_str(json).map_err(NdselError::from_serde)?;
    let kind = value
        .get("kind")
        .and_then(|k| k.as_str())
        .ok_or_else(|| NdselError::new(Reason::InvalidJson, "missing string `kind`"))?
        .to_owned();
    let msg = match kind.as_str() {
        "point" => Message::Point(from_value(value).map_err(NdselError::from_serde)?),
        "box" => Message::Box(from_value(value).map_err(NdselError::from_serde)?),
        "slice" => Message::Slice(from_value(value).map_err(NdselError::from_serde)?),
        "points" => Message::Points(from_value(value).map_err(NdselError::from_serde)?),
        "transform" => Message::Transform(from_value(value).map_err(NdselError::from_serde)?),
        other => {
            return Err(NdselError::new(Reason::UnknownKind, format!("unknown kind: {other}")));
        }
    };
    Ok(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_kind_maps_to_reason() {
        let err = parse(r#"{ "kind": "bogus" }"#).unwrap_err();
        assert_eq!(err.reason, Reason::UnknownKind);
    }

    #[test]
    fn missing_kind_is_invalid_json() {
        let err = parse(r#"{ "coords": [1] }"#).unwrap_err();
        assert_eq!(err.reason, Reason::InvalidJson);
    }

    #[test]
    fn malformed_json_is_invalid_json() {
        let err = parse(r#"{ not json"#).unwrap_err();
        assert_eq!(err.reason, Reason::InvalidJson);
    }

    #[test]
    fn integer_range_is_i64() {
        // i64 boundaries are accepted.
        parse(r#"{ "kind": "point", "coords": [9223372036854775807, -9223372036854775808] }"#)
            .and_then(normalize)
            .unwrap();
        // Just outside i64 -> invalid_json (does not fit i64 during deserialization).
        let err = parse(r#"{ "kind": "point", "coords": [9223372036854775808] }"#).unwrap_err();
        assert_eq!(err.reason, Reason::InvalidJson);
        let err = parse(r#"{ "kind": "box", "shape": [99999999999999999999] }"#)
            .and_then(normalize)
            .unwrap_err();
        assert_eq!(err.reason, Reason::InvalidJson);
    }
}
