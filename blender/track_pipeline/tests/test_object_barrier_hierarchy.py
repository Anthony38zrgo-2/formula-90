from __future__ import annotations

import json
import struct
import sys
import unittest
from pathlib import Path

import numpy as np

PIPELINE_DIR = Path(__file__).resolve().parents[1]
if str(PIPELINE_DIR) not in sys.path:
    sys.path.insert(0, str(PIPELINE_DIR))

from pipeline_common import min_distance_to_closed_polyline
from safety_barrier_layout import SafetyBarrierLayoutError, barrier_envelope_at

REPO = Path(__file__).resolve().parents[3]
CENTERLINE = REPO / "blender" / "generated" / "la_chutana" / "centerline.json"
LAYOUT = REPO / "blender" / "track_pipeline" / "manifests" / "la_chutana_safety_barriers_v3.json"
PLACEMENTS = REPO / "blender" / "generated" / "la_chutana" / "placements.json"
RUNTIME_GLB = REPO / "game" / "assets" / "generated" / "tracks" / "la_chutana" / "la_chutana.glb"

MARGIN_M = 0.5
VOLUMETRIC_RADIUS_M = {"bushes": 1.0, "trees": 1.5, "fake_buildings": 6.0}
TRACKSIDE_CARD_CLEARANCE_M = 1.0
GRASS_MIN_AXIS_DISTANCE_M = 7.5
ROAD_HALF_WIDTH_M = 6.0


def _load_json(path: Path):
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


class ObjectBarrierHierarchyTests(unittest.TestCase):
    def setUp(self) -> None:
        if not CENTERLINE.exists() or not PLACEMENTS.exists():
            self.skipTest("generated track artifacts missing; run the track pipeline first")
        center = _load_json(CENTERLINE)
        self.points = np.asarray(center["points_xz"], dtype=float)
        self.lap = float(center["length_m"])
        self.layout = _load_json(LAYOUT)

    def _outer_face(self, fraction: float, side: int):
        try:
            envelope = barrier_envelope_at(self.layout, fraction % 1.0, side, self.lap)
        except SafetyBarrierLayoutError:
            return None
        return float(envelope["outer_face_distance_m"])

    def test_volumetric_vegetation_sits_behind_active_barrier(self) -> None:
        placements = _load_json(PLACEMENTS)["placements"]
        checked = 0
        for item in placements:
            category = item.get("category")
            if category not in VOLUMETRIC_RADIUS_M:
                continue
            side = int(item.get("side", 0))
            if side == 0:
                continue
            outer = self._outer_face(float(item["track_fraction"]), side)
            if outer is None:
                continue
            required = outer + VOLUMETRIC_RADIUS_M[category] + MARGIN_M
            actual = float(item["distance_from_center_m"])
            self.assertGreaterEqual(
                actual, required,
                f"{category} at fraction {item['track_fraction']:.4f} side {side}: "
                f"{actual:.2f}m < barrier face {outer:.2f}m + clearance",
            )
            checked += 1
        self.assertGreater(checked, 0)

    def test_grass_ground_cover_stays_off_the_road(self) -> None:
        placements = _load_json(PLACEMENTS)["placements"]
        for item in placements:
            if item.get("category") != "grass":
                continue
            axis = min_distance_to_closed_polyline(
                np.asarray(item["position_xz"], dtype=float), self.points)
            self.assertGreaterEqual(
                axis, GRASS_MIN_AXIS_DISTANCE_M,
                f"grass card at {axis:.2f}m from centerline crosses the road edge",
            )
            self.assertGreaterEqual(
                axis, ROAD_HALF_WIDTH_M,
                "grass card is on the asphalt",
            )

    def test_trackside_props_clear_the_barrier_face(self) -> None:
        if not RUNTIME_GLB.exists():
            self.skipTest("runtime GLB missing; run -Mode Procedural first")
        data = RUNTIME_GLB.read_bytes()
        offset = 12
        gltf = None
        while offset < len(data):
            chunk_len, chunk_type = struct.unpack("<II", data[offset:offset + 8])
            chunk = data[offset + 8:offset + 8 + chunk_len]
            if chunk_type == 0x4E4F534A:
                gltf = json.loads(chunk.decode("utf-8"))
                break
            offset += 8 + chunk_len
        self.assertIsNotNone(gltf)
        props = []
        for node in gltf["nodes"]:
            name = node.get("name", "")
            if not name.lower().startswith("trackside") or node.get("mesh") is None:
                continue
            xs: list[float] = []
            zs: list[float] = []
            for primitive in gltf["meshes"][node["mesh"]]["primitives"]:
                accessor = gltf["accessors"][primitive["attributes"]["POSITION"]]
                xs += [accessor["min"][0], accessor["max"][0]]
                zs += [accessor["min"][2], accessor["max"][2]]
            props.append((name, (min(xs) + max(xs)) * 0.5, (min(zs) + max(zs)) * 0.5))
        self.assertGreater(len(props), 0)
        for name, cx, cz in props:
            distances = np.linalg.norm(self.points - np.array([cx, cz]), axis=1)
            nearest = int(np.argmin(distances))
            nxt = self.points[(nearest + 1) % len(self.points)]
            tangent = nxt - self.points[nearest]
            normal = np.array([-tangent[1], tangent[0]])
            side = 1 if float(np.dot([cx, cz] - self.points[nearest], normal)) > 0 else -1
            arc = float(np.sum(np.linalg.norm(
                np.diff(np.vstack([self.points[:nearest + 1],
                                   self.points[nearest + 1:]]), axis=0), axis=1)))
            fraction = arc / self.lap
            outer = self._outer_face(fraction, side)
            if outer is None:
                continue
            actual = float(min_distance_to_closed_polyline(np.array([cx, cz]), self.points))
            self.assertGreaterEqual(
                actual, outer + TRACKSIDE_CARD_CLEARANCE_M,
                f"{name} at fraction {fraction:.4f}: {actual:.2f}m < face "
                f"{outer:.2f}m + {TRACKSIDE_CARD_CLEARANCE_M}m",
            )


if __name__ == "__main__":
    unittest.main()
