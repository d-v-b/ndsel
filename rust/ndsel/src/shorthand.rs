use serde::Deserialize;

use crate::domain::{Domain, RawDomain};
use crate::error::NdselError;
use crate::output::OutputMap;
use crate::transform::{from_parts, Transform};
use crate::value::{ImplicitValue, IndexValue};

/// `{ "kind": "point", "coords": [...] }`
#[derive(Debug, Clone, Deserialize)]
pub struct Point {
    pub coords: Vec<i64>,
}

impl Point {
    pub fn desugar(self) -> Result<Transform, NdselError> {
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
    pub fn desugar(self) -> Result<Transform, NdselError> {
        let domain = self.domain.into_domain()?;
        let output = (0..domain.rank)
            .map(|k| OutputMap::SingleInputDimension { offset: 0, stride: 1, input_dimension: k })
            .collect();
        Ok(from_parts(domain, output))
    }
}

/// `{ "kind": "slice", "start": [...], "stop": [...], "step": [...]?, "labels": [...]? }`
#[derive(Debug, Clone, Deserialize)]
pub struct Slice {
    pub start: Vec<i64>,
    pub stop: Vec<i64>,
    #[serde(default)]
    pub step: Option<Vec<i64>>,
    #[serde(default)]
    pub labels: Option<Vec<String>>,
}

/// `{ "kind": "points", "coords": [[...], [...], ...] }`
#[derive(Debug, Clone, Deserialize)]
pub struct Points {
    pub coords: Vec<Vec<i64>>,
}

impl Slice {
    pub fn desugar(self) -> Result<Transform, NdselError> {
        let rank = self.start.len();
        if self.stop.len() != rank {
            return Err(NdselError::new(
                crate::error::Reason::RankMismatch,
                "start and stop must have equal length",
            ));
        }
        let step: Vec<i64> = match self.step {
            Some(s) => {
                if s.len() != rank {
                    return Err(NdselError::new(
                        crate::error::Reason::RankMismatch,
                        "step length must match start/stop",
                    ));
                }
                s
            }
            None => vec![1; rank],
        };
        if let Some(labels) = &self.labels {
            if labels.len() != rank {
                return Err(NdselError::new(
                    crate::error::Reason::RankMismatch,
                    "labels length must match start/stop",
                ));
            }
        }

        let mut inclusive_min = Vec::with_capacity(rank);
        let mut exclusive_max = Vec::with_capacity(rank);
        let mut output = Vec::with_capacity(rank);

        for k in 0..rank {
            let (lo, hi, map) = slice_dimension(self.start[k], self.stop[k], step[k], k)?;
            inclusive_min.push(lo);
            exclusive_max.push(hi);
            output.push(map);
        }

        let labels = self.labels.unwrap_or_else(|| vec![String::new(); rank]);
        let domain = Domain { rank, inclusive_min, exclusive_max, labels };
        Ok(from_parts(domain, output))
    }
}

/// One dimension of the `slice` desugaring (spec 5.3): its domain bounds and map.
fn slice_dimension(
    a: i64,
    b: i64,
    s: i64,
    k: usize,
) -> Result<(ImplicitValue, ImplicitValue, OutputMap), NdselError> {
    if s == 0 {
        return Err(NdselError::new(crate::error::Reason::StepZero, "step must be non-zero"));
    }
    // Source interval [a, b) for s > 0, [b+1, a+1) for s < 0 — see spec 5.3.
    let n = if s > 0 { b - a } else { a - b };
    if n < 0 {
        return Err(NdselError::new(
            crate::error::Reason::BoundsOutOfOrder,
            format!(
                "dimension {k}: stop must be at or after start in the direction of travel \
                 (stop >= start for step > 0, stop <= start for step < 0); \
                 got start={a}, stop={b}, step={s}"
            ),
        ));
    }
    let m = ceil_div(n, s.abs());
    let o = a / s; // trunc(a/s), both signs: Rust `/` truncates toward zero (spec 5.3)
    let offset = a - s * o; // == a % s: |offset| < |s|, with the sign of a
    let map = OutputMap::SingleInputDimension { offset, stride: s, input_dimension: k };
    Ok((fin(o), fin(o + m), map))
}

/// Ceiling of p/q for p >= 0, q > 0.
fn ceil_div(p: i64, q: i64) -> i64 {
    (p + q - 1) / q
}

impl Points {
    pub fn desugar(self) -> Result<Transform, NdselError> {
        let m = self.coords.len();
        // Output rank = point dimensionality (0 if there are no points).
        let n = self.coords.first().map(|p| p.len()).unwrap_or(0);
        for p in &self.coords {
            if p.len() != n {
                return Err(NdselError::new(
                    crate::error::Reason::RankMismatch,
                    "all points must have equal dimensionality",
                ));
            }
        }

        // input domain: single dimension [0, m)
        let domain = Domain {
            rank: 1,
            inclusive_min: vec![fin(0)],
            exclusive_max: vec![fin(m as i64)],
            labels: vec![String::new()],
        };

        // one index_array map per output dimension k: column k across all points
        let mut output = Vec::with_capacity(n);
        for k in 0..n {
            let column: Vec<serde_json::Value> = self
                .coords
                .iter()
                .map(|p| serde_json::Value::from(p[k]))
                .collect();
            output.push(OutputMap::IndexArray {
                offset: 0,
                stride: 1,
                index_array: serde_json::Value::Array(column),
                bounds: (IndexValue::NegInf, IndexValue::PosInf),
            });
        }

        Ok(from_parts(domain, output))
    }
}

/// One `outer` axis selector (spec 5.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OuterAxis {
    /// `null` — the whole axis, whatever its extent.
    All,
    /// `{"start", "stop", "step"?}` — one dimension of a `slice`.
    Range { start: i64, stop: i64, step: Option<i64> },
    /// `{"indices": [...]}` — exactly these coordinates, in order.
    Indices(Vec<i64>),
    /// `{"intervals": [[lo, hi], ...]}` — half-open intervals, concatenated in order.
    Intervals(Vec<(i64, i64)>),
}

/// `{ "kind": "outer", "axes": [...], "labels": [...]? }`
#[derive(Debug, Clone)]
pub struct Outer {
    pub axes: Vec<OuterAxis>,
    pub labels: Option<Vec<String>>,
}

/// Deserialize one member of an axis selector; a type mismatch is `invalid_json`.
fn member<T: serde::de::DeserializeOwned>(
    map: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    k: usize,
) -> Result<Option<T>, NdselError> {
    map.get(key)
        .map(|v| {
            serde_json::from_value(v.clone()).map_err(|e| {
                NdselError::new(crate::error::Reason::InvalidJson, format!("outer.axes[{k}].{key}: {e}"))
            })
        })
        .transpose()
}

impl OuterAxis {
    fn from_json(value: &serde_json::Value, k: usize) -> Result<Self, NdselError> {
        use crate::error::Reason;
        let map = match value {
            serde_json::Value::Null => return Ok(OuterAxis::All),
            serde_json::Value::Object(map) => map,
            other => {
                return Err(NdselError::new(
                    Reason::InvalidJson,
                    format!("outer.axes[{k}] must be null or an object, got {other}"),
                ));
            }
        };
        // Untagged, like an output map: the members present pick the form
        // (indices > intervals > range), which then has a closed member set.
        let (form, allowed): (&str, &[&str]) = if map.contains_key("indices") {
            ("indices", &["indices"])
        } else if map.contains_key("intervals") {
            ("intervals", &["intervals"])
        } else {
            ("range", &["start", "stop", "step"])
        };
        if let Some(key) = map.keys().find(|key| !allowed.contains(&key.as_str())) {
            return Err(NdselError::new(
                Reason::UnknownField,
                format!("outer.axes[{k}]: unrecognized field for a {form} selector: {key}"),
            ));
        }
        match form {
            "indices" => Ok(OuterAxis::Indices(member(map, "indices", k)?.unwrap())),
            "intervals" => {
                let pairs: Vec<Vec<i64>> = member(map, "intervals", k)?.unwrap();
                let intervals = pairs
                    .into_iter()
                    .enumerate()
                    .map(|(j, pair)| match pair[..] {
                        [lo, hi] => Ok((lo, hi)),
                        _ => Err(NdselError::new(
                            Reason::InvalidJson,
                            format!("outer.axes[{k}].intervals[{j}] must be a [lo, hi] pair"),
                        )),
                    })
                    .collect::<Result<_, _>>()?;
                Ok(OuterAxis::Intervals(intervals))
            }
            _ => {
                let (Some(start), Some(stop)) = (member(map, "start", k)?, member(map, "stop", k)?) else {
                    return Err(NdselError::new(
                        Reason::InvalidJson,
                        format!("outer.axes[{k}]: a range needs start and stop"),
                    ));
                };
                Ok(OuterAxis::Range { start, stop, step: member(map, "step", k)? })
            }
        }
    }
}

impl Outer {
    /// Build from a parsed message object (top-level members already checked by `parse`).
    pub(crate) fn from_json(value: serde_json::Value) -> Result<Self, NdselError> {
        use crate::error::Reason;
        let Some(serde_json::Value::Array(raw_axes)) = value.get("axes") else {
            return Err(NdselError::new(Reason::InvalidJson, "outer.axes must be an array"));
        };
        let axes = raw_axes
            .iter()
            .enumerate()
            .map(|(k, v)| OuterAxis::from_json(v, k))
            .collect::<Result<_, _>>()?;
        let labels = value
            .get("labels")
            .map(|v| serde_json::from_value(v.clone()).map_err(NdselError::from_serde))
            .transpose()?;
        Ok(Outer { axes, labels })
    }

    pub fn desugar(self) -> Result<Transform, NdselError> {
        let rank = self.axes.len();
        if let Some(labels) = &self.labels {
            if labels.len() != rank {
                return Err(NdselError::new(
                    crate::error::Reason::RankMismatch,
                    "labels length must match axes",
                ));
            }
        }
        let mut inclusive_min = Vec::with_capacity(rank);
        let mut exclusive_max = Vec::with_capacity(rank);
        let mut output = Vec::with_capacity(rank);
        for (k, axis) in self.axes.into_iter().enumerate() {
            let coords = match axis {
                OuterAxis::All => {
                    // The whole axis: implicit infinite bounds, resolved against the array.
                    inclusive_min.push(ImplicitValue { value: IndexValue::NegInf, implicit: true });
                    exclusive_max.push(ImplicitValue { value: IndexValue::PosInf, implicit: true });
                    output.push(OutputMap::SingleInputDimension { offset: 0, stride: 1, input_dimension: k });
                    continue;
                }
                OuterAxis::Range { start, stop, step } => {
                    let (lo, hi, map) = slice_dimension(start, stop, step.unwrap_or(1), k)?;
                    inclusive_min.push(lo);
                    exclusive_max.push(hi);
                    output.push(map);
                    continue;
                }
                OuterAxis::Indices(coords) => coords,
                OuterAxis::Intervals(intervals) => expand_intervals(&intervals, k)?,
            };
            inclusive_min.push(fin(0));
            exclusive_max.push(fin(coords.len() as i64));
            // Empty and single-element lists collapse to constant maps (spec 7.1).
            output.push(match coords[..] {
                [] => OutputMap::Constant { offset: 0 },
                [c] => OutputMap::Constant { offset: c },
                _ => OutputMap::IndexArray {
                    offset: 0,
                    stride: 1,
                    index_array: broadcast(&coords, rank, k),
                    bounds: (IndexValue::NegInf, IndexValue::PosInf),
                },
            });
        }
        let labels = self.labels.unwrap_or_else(|| vec![String::new(); rank]);
        let domain = Domain { rank, inclusive_min, exclusive_max, labels };
        Ok(from_parts(domain, output))
    }
}

fn expand_intervals(intervals: &[(i64, i64)], k: usize) -> Result<Vec<i64>, NdselError> {
    let mut coords = Vec::new();
    for (j, &(lo, hi)) in intervals.iter().enumerate() {
        if lo > hi {
            return Err(NdselError::new(
                crate::error::Reason::BoundsOutOfOrder,
                format!("outer.axes[{k}].intervals[{j}]: lo {lo} exceeds hi {hi}"),
            ));
        }
        coords.extend(lo..hi);
    }
    Ok(coords)
}

/// `coords` reshaped to the broadcast shape: `coords.len()` in dimension `k`, 1 elsewhere.
fn broadcast(coords: &[i64], rank: usize, k: usize) -> serde_json::Value {
    use serde_json::Value;
    let leaves = coords.iter().map(|&c| {
        (k + 1..rank).fold(Value::from(c), |v, _| Value::Array(vec![v]))
    });
    (0..k).fold(Value::Array(leaves.collect()), |v, _| Value::Array(vec![v]))
}

/// An explicit finite bound, used by the slice/points desugarers.
fn fin(n: i64) -> ImplicitValue { ImplicitValue::explicit(IndexValue::Finite(n)) }

// ---------------------------------------------------------------------------
// Ergonomic builders — parity with the Python (`Box(shape=[3, 4])`) and
// TypeScript (`box({ shape: [3, 4] })`) construction APIs. Build a message,
// then `normalize(msg.into())` (each builder is `Into<Message>`). Bound setters
// take plain `i64` for the common finite case; for `±inf` or implicit bounds,
// use `parse` on a JSON string.
// ---------------------------------------------------------------------------

/// Map finite integers to explicit bounds.
fn finite_bounds(values: Vec<i64>) -> Vec<ImplicitValue> {
    values.into_iter().map(|n| ImplicitValue::explicit(IndexValue::Finite(n))).collect()
}

impl Point {
    /// `{ "kind": "point", "coords": [...] }` — select one grid point per dimension.
    pub fn new(coords: Vec<i64>) -> Self {
        Point { coords }
    }
}

impl Points {
    /// `{ "kind": "points", "coords": [[...], ...] }` — an explicit set of points.
    pub fn new(coords: Vec<Vec<i64>>) -> Self {
        Points { coords }
    }
}

impl Outer {
    /// `{ "kind": "outer", "axes": [...] }` — one selector per dimension.
    pub fn new(axes: Vec<OuterAxis>) -> Self {
        Outer { axes, labels: None }
    }

    /// Set per-dimension labels.
    pub fn labels(mut self, labels: Vec<String>) -> Self {
        self.labels = Some(labels);
        self
    }
}

impl Slice {
    /// `{ "kind": "slice", "start": [...], "stop": [...] }` (step defaults to 1).
    pub fn new(start: Vec<i64>, stop: Vec<i64>) -> Self {
        Slice { start, stop, step: None, labels: None }
    }

    /// Set a per-dimension step.
    pub fn step(mut self, step: Vec<i64>) -> Self {
        self.step = Some(step);
        self
    }

    /// Set per-dimension labels.
    pub fn labels(mut self, labels: Vec<String>) -> Self {
        self.labels = Some(labels);
        self
    }
}

impl BoxSel {
    /// An empty box; choose one upper-bound spelling with the setters below.
    pub fn new() -> Self {
        BoxSel { domain: RawDomain::default() }
    }

    /// Set the inclusive lower bound (defaults to all-zero when omitted).
    pub fn inclusive_min(mut self, values: Vec<i64>) -> Self {
        self.domain.inclusive_min = Some(finite_bounds(values));
        self
    }

    /// Set the exclusive upper bound.
    pub fn exclusive_max(mut self, values: Vec<i64>) -> Self {
        self.domain.exclusive_max = Some(finite_bounds(values));
        self
    }

    /// Set the inclusive upper bound (canonicalized to `exclusive_max`).
    pub fn inclusive_max(mut self, values: Vec<i64>) -> Self {
        self.domain.inclusive_max = Some(finite_bounds(values));
        self
    }

    /// Set the per-dimension extent (`exclusive_max = inclusive_min + shape`).
    pub fn shape(mut self, values: Vec<i64>) -> Self {
        self.domain.shape = Some(finite_bounds(values));
        self
    }

    /// Set per-dimension labels.
    pub fn labels(mut self, labels: Vec<String>) -> Self {
        self.domain.labels = Some(labels);
        self
    }
}

impl Default for BoxSel {
    fn default() -> Self {
        BoxSel::new()
    }
}

#[cfg(test)]
mod builder_tests {
    use crate::{BoxSel, Message, Point, Points, Slice, normalize, parse};

    fn built(m: Message) -> serde_json::Value {
        serde_json::to_value(normalize(m).unwrap()).unwrap()
    }
    fn parsed(json: &str) -> serde_json::Value {
        serde_json::to_value(normalize(parse(json).unwrap()).unwrap()).unwrap()
    }

    #[test]
    fn point_builder_matches_parse() {
        assert_eq!(
            built(Point::new(vec![4, 7]).into()),
            parsed(r#"{"kind":"point","coords":[4,7]}"#)
        );
    }

    #[test]
    fn box_builder_matches_parse() {
        assert_eq!(
            built(BoxSel::new().shape(vec![3, 4]).into()),
            parsed(r#"{"kind":"box","shape":[3,4]}"#)
        );
        assert_eq!(
            built(BoxSel::new().inclusive_min(vec![0, 0]).exclusive_max(vec![3, 4]).into()),
            parsed(r#"{"kind":"box","inclusive_min":[0,0],"exclusive_max":[3,4]}"#)
        );
    }

    #[test]
    fn slice_builder_matches_parse() {
        assert_eq!(
            built(Slice::new(vec![0], vec![10]).step(vec![2]).into()),
            parsed(r#"{"kind":"slice","start":[0],"stop":[10],"step":[2]}"#)
        );
    }

    #[test]
    fn outer_builder_matches_parse() {
        use crate::{Outer, OuterAxis};
        assert_eq!(
            built(
                Outer::new(vec![OuterAxis::All, OuterAxis::Intervals(vec![(0, 3)])])
                    .labels(vec!["y".into(), "x".into()])
                    .into()
            ),
            parsed(r#"{"kind":"outer","axes":[null,{"intervals":[[0,3]]}],"labels":["y","x"]}"#)
        );
    }

    #[test]
    fn points_builder_matches_parse() {
        assert_eq!(
            built(Points::new(vec![vec![1, 10], vec![2, 20]]).into()),
            parsed(r#"{"kind":"points","coords":[[1,10],[2,20]]}"#)
        );
    }
}

#[cfg(test)]
mod slice_tests {

    fn norm(json: &str) -> serde_json::Value {
        let msg = crate::parse(json).unwrap();
        serde_json::to_value(crate::normalize(msg).unwrap()).unwrap()
    }
    fn err(json: &str) -> crate::Reason {
        let msg = crate::parse(json).unwrap();
        crate::normalize(msg).unwrap_err().reason
    }

    #[test]
    fn unit_step_preserves_source_frame() {
        // x[5:10] -> domain [5,10), identity map (coordinate-preserving)
        let v = norm(r#"{ "kind": "slice", "start": [5], "stop": [10] }"#);
        assert_eq!(v["input_inclusive_min"], serde_json::json!([5]));
        assert_eq!(v["input_exclusive_max"], serde_json::json!([10]));
        assert_eq!(
            v["output"],
            serde_json::json!([{ "offset": 0, "stride": 1, "input_dimension": 0 }])
        );
    }

    #[test]
    fn divisible_strided_slice() {
        // x[4:10:2] selects {4,6,8}; o=2, offset=0, domain [2,5)
        let v = norm(r#"{ "kind": "slice", "start": [4], "stop": [10], "step": [2] }"#);
        assert_eq!(v["input_inclusive_min"], serde_json::json!([2]));
        assert_eq!(v["input_exclusive_max"], serde_json::json!([5]));
        assert_eq!(
            v["output"],
            serde_json::json!([{ "offset": 0, "stride": 2, "input_dimension": 0 }])
        );
    }

    #[test]
    fn nondivisible_strided_slice_uses_phase_offset() {
        // x[5:10:2] selects {5,7,9}; o=floor(5/2)=2, offset=5-2*2=1, domain [2,5)
        let v = norm(r#"{ "kind": "slice", "start": [5], "stop": [10], "step": [2] }"#);
        assert_eq!(v["input_inclusive_min"], serde_json::json!([2]));
        assert_eq!(v["input_exclusive_max"], serde_json::json!([5]));
        assert_eq!(
            v["output"],
            serde_json::json!([{ "offset": 1, "stride": 2, "input_dimension": 0 }])
        );
    }

    #[test]
    fn empty_slice_has_zero_length_dim() {
        // x[10:10] -> m=0, o=10, domain [10,10)
        let v = norm(r#"{ "kind": "slice", "start": [10], "stop": [10] }"#);
        assert_eq!(v["input_inclusive_min"], serde_json::json!([10]));
        assert_eq!(v["input_exclusive_max"], serde_json::json!([10]));
    }

    #[test]
    fn step_zero_is_error() {
        assert_eq!(err(r#"{ "kind": "slice", "start": [0], "stop": [4], "step": [0] }"#), crate::Reason::StepZero);
    }

    /// Negative steps desugar by the same rule as positive ones (spec 5.3).
    /// `(start, stop, step) -> (inclusive_min, exclusive_max, offset)`.
    #[test]
    fn negative_step_slices() {
        // x[19:-1:-1] — a reversed length-20 axis: domain [-19, 1), points 19..0
        // x[15:5:-2]  — divisible span: points 15,13,11,9,7
        // x[15:5:-4]  — span 10 is not divisible by 4: points 15,11,7
        // x[-1:-6:-2] — negative interval; trunc(-1/-2) = 0, so the origin is 0
        // x[5:4:-3]   — one point
        // x[5:5:-1]   — empty is legal anywhere, at the origin trunc(5/-1) = -5
        let cases: &[(i64, i64, i64, i64, i64, i64)] = &[
            (19, -1, -1, -19, 1, 0),
            (15, 5, -2, -7, -2, 1),
            (15, 5, -4, -3, 0, 3),
            (-1, -6, -2, 0, 3, -1),
            (5, 4, -3, -1, 0, 2),
            (5, 5, -1, -5, -5, 0),
        ];
        for &(start, stop, step, lo, hi, offset) in cases {
            let v = norm(&format!(
                r#"{{ "kind": "slice", "start": [{start}], "stop": [{stop}], "step": [{step}] }}"#
            ));
            assert_eq!(v["input_inclusive_min"], serde_json::json!([lo]), "case {start}:{stop}:{step}");
            assert_eq!(v["input_exclusive_max"], serde_json::json!([hi]), "case {start}:{stop}:{step}");
            assert_eq!(
                v["output"],
                serde_json::json!([{ "offset": offset, "stride": step, "input_dimension": 0 }]),
                "case {start}:{stop}:{step}"
            );
        }
    }

    #[test]
    fn negative_step_keeps_labels() {
        let v = norm(
            r#"{ "kind": "slice", "start": [19], "stop": [-1], "step": [-1], "labels": ["x"] }"#,
        );
        assert_eq!(v["input_labels"], serde_json::json!(["x"]));
    }

    #[test]
    fn reversed_interval_positive_step_is_error() {
        assert_eq!(
            err(r#"{ "kind": "slice", "start": [9], "stop": [0], "step": [2] }"#),
            crate::Reason::BoundsOutOfOrder
        );
    }

    #[test]
    fn reversed_interval_negative_step_is_error() {
        assert_eq!(
            err(r#"{ "kind": "slice", "start": [5], "stop": [6], "step": [-1] }"#),
            crate::Reason::BoundsOutOfOrder
        );
    }

    #[test]
    fn slice_length_mismatch_is_rank_mismatch() {
        assert_eq!(
            err(r#"{ "kind": "slice", "start": [0, 0], "stop": [4] }"#),
            crate::Reason::RankMismatch
        );
    }
}

#[cfg(test)]
mod points_tests {

    fn norm(json: &str) -> serde_json::Value {
        let msg = crate::parse(json).unwrap();
        serde_json::to_value(crate::normalize(msg).unwrap()).unwrap()
    }

    #[test]
    fn points_transpose_to_columnar_index_arrays() {
        // three 2-D points: (1,10), (2,20), (3,30)
        let v = norm(r#"{ "kind": "points", "coords": [[1, 10], [2, 20], [3, 30]] }"#);
        assert_eq!(v["input_rank"], 1);
        assert_eq!(v["input_inclusive_min"], serde_json::json!([0]));
        assert_eq!(v["input_exclusive_max"], serde_json::json!([3]));
        assert_eq!(
            v["output"],
            serde_json::json!([
                { "offset": 0, "stride": 1, "index_array": [1, 2, 3],   "index_array_bounds": ["-inf", "+inf"] },
                { "offset": 0, "stride": 1, "index_array": [10, 20, 30], "index_array_bounds": ["-inf", "+inf"] }
            ])
        );
    }

    #[test]
    fn empty_points_is_zero_length_with_unknown_rank_zero() {
        // No points -> m=0, output rank 0 (no columns to emit)
        let v = norm(r#"{ "kind": "points", "coords": [] }"#);
        assert_eq!(v["input_rank"], 1);
        assert_eq!(v["input_exclusive_max"], serde_json::json!([0]));
        assert_eq!(v["output"], serde_json::json!([]));
    }

    #[test]
    fn ragged_points_is_rank_mismatch() {
        let msg = crate::parse(r#"{ "kind": "points", "coords": [[1, 2], [3]] }"#).unwrap();
        assert_eq!(crate::normalize(msg).unwrap_err().reason, crate::Reason::RankMismatch);
    }
}

#[cfg(test)]
mod point_box_tests {

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

#[cfg(test)]
mod outer_tests {
    use serde_json::json;

    fn norm(json: &str) -> serde_json::Value {
        let msg = crate::parse(json).unwrap();
        serde_json::to_value(crate::normalize(msg).unwrap()).unwrap()
    }
    fn err(json: &str) -> crate::Reason {
        match crate::parse(json) {
            Ok(msg) => crate::normalize(msg).unwrap_err().reason,
            Err(e) => e.reason,
        }
    }

    /// `axes -> (input_inclusive_min, input_exclusive_max, output)`.
    #[test]
    fn outer_desugaring() {
        let ia = |a: serde_json::Value| {
            json!({ "offset": 0, "stride": 1, "index_array": a, "index_array_bounds": ["-inf", "+inf"] })
        };
        let id = |k: usize| json!({ "offset": 0, "stride": 1, "input_dimension": k });
        let cases = [
            // The whole axis: implicit infinite bounds, identity map.
            (json!([null]), json!([["-inf"]]), json!([["+inf"]]), json!([id(0)])),
            // A range is one dimension of a `slice`: x[5:10:2] keeps its source frame.
            (
                json!([{ "start": 5, "stop": 10, "step": 2 }]),
                json!([2]),
                json!([5]),
                json!([{ "offset": 1, "stride": 2, "input_dimension": 0 }]),
            ),
            // Intervals expand in order; the result is numbered [0, m).
            (
                json!([{ "intervals": [[0, 2], [5, 7]] }]),
                json!([0]),
                json!([4]),
                json!([ia(json!([0, 1, 5, 6]))]),
            ),
            // The array broadcasts: m along its own dimension, 1 along every other.
            (
                json!([null, { "indices": [4, 2] }, null]),
                json!([["-inf"], 0, ["-inf"]]),
                json!([["+inf"], 2, ["+inf"]]),
                json!([id(0), ia(json!([[[4], [2]]])), id(2)]),
            ),
            // One coordinate collapses to a constant map; none collapses to offset 0.
            (json!([{ "indices": [7] }]), json!([0]), json!([1]), json!([{ "offset": 7 }])),
            (json!([{ "intervals": [[3, 3]] }]), json!([0]), json!([0]), json!([{ "offset": 0 }])),
        ];
        for (axes, lo, hi, output) in cases {
            let v = norm(&json!({ "kind": "outer", "axes": axes }).to_string());
            assert_eq!(v["input_inclusive_min"], lo, "axes {axes}");
            assert_eq!(v["input_exclusive_max"], hi, "axes {axes}");
            assert_eq!(v["output"], output, "axes {axes}");
        }
    }

    #[test]
    fn selector_with_members_of_two_forms_is_unknown_field() {
        assert_eq!(
            err(r#"{"kind":"outer","axes":[{"indices":[1],"intervals":[[0,1]]}]}"#),
            crate::Reason::UnknownField
        );
    }

    #[test]
    fn range_missing_stop_is_invalid_json() {
        assert_eq!(err(r#"{"kind":"outer","axes":[{"start":0}]}"#), crate::Reason::InvalidJson);
    }

    #[test]
    fn interval_that_is_not_a_pair_is_invalid_json() {
        assert_eq!(err(r#"{"kind":"outer","axes":[{"intervals":[[1,2,3]]}]}"#), crate::Reason::InvalidJson);
    }

    #[test]
    fn reversed_interval_is_bounds_out_of_order() {
        assert_eq!(err(r#"{"kind":"outer","axes":[{"intervals":[[5,3]]}]}"#), crate::Reason::BoundsOutOfOrder);
    }

    #[test]
    fn range_step_zero() {
        assert_eq!(err(r#"{"kind":"outer","axes":[{"start":0,"stop":4,"step":0}]}"#), crate::Reason::StepZero);
    }

    #[test]
    fn labels_length_mismatch_is_rank_mismatch() {
        assert_eq!(err(r#"{"kind":"outer","axes":[null,null],"labels":["x"]}"#), crate::Reason::RankMismatch);
    }
}
