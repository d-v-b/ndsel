"""Runnable example usage of the ndsel Python API.

    cd python/ndsel && uv run python examples/usage.py
"""

import json

from ndsel import Box, NdselError, Point, Points, Slice, normalize, parse


def show(label: str, value: object) -> None:
    print(f"  {label:18} {json.dumps(value)}")


print("# parse a JSON message, then normalize to the canonical transform")
show("slice [10:20:2]", normalize(parse('{"kind":"slice","start":[10],"stop":[20],"step":[2]}')).to_json())

print("\n# build each kind programmatically (ergonomic dataclass builders)")
show("point (4, 7)", normalize(Point(coords=[4, 7])).to_json())
show("box shape 3x4", normalize(Box(shape=[3, 4])).to_json())
show("slice [0:10:2]", normalize(Slice(start=[0], stop=[10], step=[2])).to_json())
show("points", normalize(Points(coords=[[1, 10], [2, 20]])).to_json())
show("transform 3x4", normalize(parse('{"kind":"transform","input_inclusive_min":[0,0],"input_exclusive_max":[3,4]}')).to_json())

print("\n# error handling: stable, machine-readable reason codes")
for bad in (
    '{"kind":"wat"}',
    '{"kind":"slice","start":[0],"stop":[10],"step":[0]}',
    '{"kind":"slice","start":[0],"stop":[10],"step":[-1]}',
):
    try:
        normalize(parse(bad))
    except NdselError as e:
        print(f"  {bad:52} -> {e.reason.value}")

print("\n# edge cases")
show("big i64 (2**60)", normalize(Point(coords=[2**60])).to_json())
show("unbounded box", normalize(Box(inclusive_min=[0], exclusive_max=["+inf"])).to_json())
