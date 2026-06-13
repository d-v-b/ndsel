//! ndsq — JSON-serializable n-dimensional spatial queries.
//!
//! A `Message` is a `kind`-discriminated union; `normalize` reduces any message
//! to a canonical `Transform`. Adapts the index model of Google tensorstore.
//!
//! The public API is wired up in `shorthand` (Task 9) once every module exists.

mod domain;
mod error;
mod output;
mod shorthand;
mod transform;
mod value;
