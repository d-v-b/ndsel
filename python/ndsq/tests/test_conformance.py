"""Runs the repo-root /conformance corpus against the Python implementation."""

from __future__ import annotations

import glob
import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

from ndsq import NdsqError, normalize, parse

REPO_ROOT = Path(__file__).parents[3]
CORPUS_DIR = REPO_ROOT / "conformance"
SCHEMA_PATH = REPO_ROOT / "schema" / "ndsq.schema.json"


def _load_cases() -> list[tuple[str, dict]]:
    cases: list[tuple[str, dict]] = []
    for path in sorted(glob.glob(str(CORPUS_DIR / "*.json"))):
        for case in json.loads(Path(path).read_text()):
            cases.append((case.get("name", "<unnamed>"), case))
    return cases


_VALIDATOR = Draft202012Validator(json.loads(SCHEMA_PATH.read_text()))
_CASES = _load_cases()


def test_corpus_is_populated():
    assert len(_CASES) >= 15, f"expected a populated corpus, found {len(_CASES)}"


@pytest.mark.parametrize("name,case", _CASES, ids=[c[0] for c in _CASES])
def test_corpus_case(name: str, case: dict):
    input_msg = case["input"]
    if "error" in case:
        # Error inputs may be intentionally schema-invalid; not schema-checked.
        with pytest.raises(NdsqError) as exc:
            normalize(parse(json.dumps(input_msg)))
        assert exc.value.reason.value == case["error"]
    else:
        assert _VALIDATOR.is_valid(input_msg), f"{name}: input fails schema"
        got = normalize(parse(json.dumps(input_msg))).to_dict()
        assert got == case["normalized"]
