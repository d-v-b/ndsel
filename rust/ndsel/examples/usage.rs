//! Runnable example usage of the ndsel Rust API.
//!
//!     cd rust/ndsel && cargo run --example usage
//!
//! Messages can be built programmatically with the ergonomic constructors
//! (`Point::new`, `BoxSel::new().shape(...)`, `Slice::new(...).step(...)`,
//! `Points::new`) — each is `Into<Message>` — or parsed from JSON with `parse`.
//! Either way, `normalize` reduces a message to a canonical `Transform`, which
//! serializes through serde (`serde_json::to_string`).

use ndsel::{BoxSel, NdselError, Point, Points, Slice, Transform, normalize, parse};

fn show(label: &str, result: Result<Transform, NdselError>) {
    match result {
        Ok(t) => println!("  {label:18} {}", serde_json::to_string(&t).unwrap()),
        Err(e) => println!("  {label:18} ERROR {e}"),
    }
}

fn main() {
    println!("# parse a JSON message, then normalize to the canonical transform");
    show("slice [10:20:2]", parse(r#"{"kind":"slice","start":[10],"stop":[20],"step":[2]}"#).and_then(normalize));

    println!("\n# build each kind programmatically (ergonomic constructors)");
    show("point (4, 7)", normalize(Point::new(vec![4, 7]).into()));
    show("box shape 3x4", normalize(BoxSel::new().shape(vec![3, 4]).into()));
    show("slice [0:10:2]", normalize(Slice::new(vec![0], vec![10]).step(vec![2]).into()));
    show("points", normalize(Points::new(vec![vec![1, 10], vec![2, 20]]).into()));
    show("transform 3x4", parse(r#"{"kind":"transform","input_inclusive_min":[0,0],"input_exclusive_max":[3,4]}"#).and_then(normalize));

    println!("\n# error handling: stable, machine-readable reason codes");
    for bad in [
        r#"{"kind":"wat"}"#,
        r#"{"kind":"slice","start":[0],"stop":[10],"step":[0]}"#,
        r#"{"kind":"slice","start":[0],"stop":[10],"step":[-1]}"#,
    ] {
        match parse(bad).and_then(normalize) {
            Ok(_) => println!("  {bad:52} -> OK (unexpected)"),
            Err(e) => println!("  {bad:52} -> {}", e.reason.code()),
        }
    }

    println!("\n# edge cases");
    show("big i64 (2**60)", normalize(Point::new(vec![1152921504606846976]).into()));
    // `±inf` bounds are not expressible through the finite-i64 setters; use parse.
    show("unbounded box", parse(r#"{"kind":"box","inclusive_min":[0],"exclusive_max":["+inf"]}"#).and_then(normalize));
}
