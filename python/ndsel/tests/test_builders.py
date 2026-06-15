from ndsel.builders import Box, Point, Points, Slice


def test_point_to_json():
    assert Point(coords=[4, 7]).to_json() == {"kind": "point", "coords": [4, 7]}


def test_box_omits_unset_fields():
    assert Box(inclusive_min=[0, 0], exclusive_max=[3, 4]).to_json() == {
        "kind": "box",
        "inclusive_min": [0, 0],
        "exclusive_max": [3, 4],
    }
    assert Box(shape=[5]).to_json() == {"kind": "box", "shape": [5]}


def test_slice_omits_unset_step_and_labels():
    assert Slice(start=[0], stop=[10]).to_json() == {
        "kind": "slice",
        "start": [0],
        "stop": [10],
    }
    assert Slice(start=[0], stop=[10], step=[2]).to_json() == {
        "kind": "slice",
        "start": [0],
        "stop": [10],
        "step": [2],
    }


def test_points_to_json():
    assert Points(coords=[[1, 10], [2, 20]]).to_json() == {
        "kind": "points",
        "coords": [[1, 10], [2, 20]],
    }
