# ndsel

Python peer implementation of **ndsel** — a JSON-serializable representation of n-dimensional array indexing ("spatial query"). It passes the same shared conformance corpus as the Rust reference.

## Two-layer model

- **TypedDicts** (`messages.py`) model the raw JSON messages exactly, giving a 1-to-1 mapping between Python dicts and the wire format.
- **Dataclasses** provide ergonomic input builders — `Point`, `Box`, `Slice`, `Points` — and the canonical result type `Transform` (with a `.to_dict()` method that serializes back to the JSON representation).

## Usage

```python
from ndsel import normalize, parse, Box

# from JSON
normalize(parse('{"kind": "slice", "start": [0], "stop": [10], "step": [2]}')).to_dict()

# from a builder dataclass
normalize(Box(inclusive_min=[0, 0], exclusive_max=[3, 4])).to_dict()
```

The smoke-test output for a unit box:

```python
>>> from ndsel import normalize, Box
>>> normalize(Box(shape=[3])).to_dict()
{'input_rank': 1, 'input_inclusive_min': [0], 'input_exclusive_max': [3], 'input_labels': [''], 'output': [{'offset': 0, 'stride': 1, 'input_dimension': 0}]}
```

## Index model

ndsel adapts [TensorStore's index model](../../spec/ndsel.md). All inputs are normalized to a canonical `Transform` that matches the spec.

## Validation

The implementation is validated against the shared conformance corpus in [`../../conformance/`](../../conformance/). Every fixture is exercised by the test suite.

## Develop

```
uv run pytest
```

Runs both the unit tests and the full conformance suite.

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT)
at your option (`MIT OR Apache-2.0`). The ndsel specification itself is public domain
([CC0](../../LICENSE-CC0)).
