//! Runs every fixture in ../../conformance against `ndsel::normalize`,
//! and validates each `input` against the JSON Schema.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

fn repo_root() -> PathBuf {
    // tests run with CWD = rust/ndsel; corpus + schema are two levels up.
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn load_schema() -> jsonschema::Validator {
    let path = repo_root().join("schema/ndsel.schema.json");
    let schema: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    jsonschema::validator_for(&schema).expect("schema compiles")
}

fn corpus_files() -> Vec<PathBuf> {
    let dir = repo_root().join("conformance");
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "json").unwrap_or(false))
        .collect();
    files.sort();
    files
}

/// Strip the `kind` field for comparison: corpus `normalized` omits it.
fn without_kind(mut v: Value) -> Value {
    if let Value::Object(map) = &mut v {
        map.remove("kind");
    }
    v
}

#[test]
fn corpus_round_trips() {
    let schema = load_schema();
    let mut count = 0usize;

    for file in corpus_files() {
        let cases: Vec<Value> =
            serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        for case in cases {
            let name = case["name"].as_str().unwrap_or("<unnamed>");
            let input = &case["input"];
            let msg = ndsel::parse(&input.to_string());

            if let Some(expected_reason) = case.get("error").and_then(|e| e.as_str()) {
                // Error inputs may be intentionally schema-invalid, so they are
                // NOT schema-checked here.
                let err = match msg {
                    Ok(m) => ndsel::normalize(m).expect_err(&format!("{name}: expected error")),
                    Err(e) => e,
                };
                assert_eq!(err.reason.code(), expected_reason, "{}: wrong reason", name);
            } else {
                // Every success input MUST satisfy the schema (syntax).
                assert!(schema.is_valid(input), "{}: input fails schema", name);
                let normalized = ndsel::normalize(msg.expect(name)).expect(name);
                let got = without_kind(serde_json::to_value(&normalized).unwrap());
                let want = case["normalized"].clone();
                assert_eq!(got, want, "{}: normalized mismatch", name);
            }
            count += 1;
        }
    }
    assert!(count >= 15, "expected a populated corpus, ran {count}");
}
