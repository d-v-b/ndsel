"""The canonical core: a Transform (domain + explicit output maps)."""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from .domain import Domain, canonicalize_domain
from .messages import NormalizedTransform
from .output import OutputMap, SingleInputDimension, canonicalize_output_map, output_map_to_json
from .values import require_list


@dataclass(frozen=True, slots=True, kw_only=True)
class Transform:
    """The canonical core. Serialize with `to_dict()` (the bare transform body, no `kind`)."""

    domain: Domain
    output: Sequence[OutputMap]

    def to_dict(self) -> NormalizedTransform:
        return {
            **self.domain.to_json_fields(),
            "output": [output_map_to_json(m) for m in self.output],
        }


def identity_output(rank: int) -> list[OutputMap]:
    return [SingleInputDimension(offset=0, stride=1, input_dimension=k) for k in range(rank)]


def canonicalize_transform(msg: Mapping[str, object]) -> Transform:
    """Canonicalize a `transform` message body (uses the input_-prefixed field names).

    ``msg`` is a read-only Mapping rather than a concrete TypedDict on purpose: this
    canonicalizes a transform *body* and accepts several structurally different shapes —
    the wire `TransformMessage` `normalize` passes (``kind`` present), the kind-less
    `NormalizedTransform` it emits (re-fed for idempotency), and partial bodies. TypedDict
    assignability requires the source to declare a superset of the target's keys, so no
    single TypedDict is a supertype of all three; a read-only Mapping is. ``kind`` is never
    read here.
    """
    domain = canonicalize_domain(
        rank=msg.get("input_rank"),
        inclusive_min=msg.get("input_inclusive_min"),
        exclusive_max=msg.get("input_exclusive_max"),
        inclusive_max=msg.get("input_inclusive_max"),
        shape=msg.get("input_shape"),
        labels=msg.get("input_labels"),
    )
    raw_output = msg.get("output")
    if raw_output is not None:
        output = [canonicalize_output_map(m) for m in require_list(raw_output, "output")]
    else:
        output = identity_output(domain.rank)
    return Transform(domain=domain, output=output)
