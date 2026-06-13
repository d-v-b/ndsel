//! ndsq — JSON-serializable n-dimensional spatial queries.
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
pub use error::{NdsqError, Reason};
pub use output::OutputMap;
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

/// Reduce any message to its canonical `Transform`.
pub fn normalize(message: Message) -> Result<Transform, NdsqError> {
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
pub fn parse(json: &str) -> Result<Message, NdsqError> {
    use serde_json::{from_value, Value};
    let value: Value = serde_json::from_str(json).map_err(NdsqError::from_serde)?;
    let kind = value
        .get("kind")
        .and_then(|k| k.as_str())
        .ok_or_else(|| NdsqError::new(Reason::InvalidJson, "missing string `kind`"))?
        .to_owned();
    let msg = match kind.as_str() {
        "point" => Message::Point(from_value(value).map_err(NdsqError::from_serde)?),
        "box" => Message::Box(from_value(value).map_err(NdsqError::from_serde)?),
        "slice" => Message::Slice(from_value(value).map_err(NdsqError::from_serde)?),
        "points" => Message::Points(from_value(value).map_err(NdsqError::from_serde)?),
        "transform" => Message::Transform(from_value(value).map_err(NdsqError::from_serde)?),
        other => {
            return Err(NdsqError::new(Reason::UnknownKind, format!("unknown kind: {other}")));
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
}
