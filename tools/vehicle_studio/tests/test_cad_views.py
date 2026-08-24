from __future__ import annotations

from copy import deepcopy
from hashlib import sha256
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from vehicle_studio.cad_views import cad_manifest_bytes, compile_cad_views
from vehicle_studio.domain import VehicleDocument


FIXTURE = "blender/track_pipeline/tests/fixtures/assets/valid_tree.glb"


def document() -> VehicleDocument:
    data = {
        "schema_version": 1, "project_id": "cad-test", "revision_id": "revision-zero-test",
        "source": {"kind": "glb", "path": FIXTURE, "size_bytes": 1, "sha256": "A" * 64, "evaluated_geometry_sha256": "A" * 64, "blender_version": None, "provenance_status": "TEST"},
        "coordinate_system": {"unit": "meter", "right_axis": "+X", "up_axis": "+Y", "forward_axis": "-Z", "ground_y_m": 0},
        "component_map": [{"component_id": "component-cube", "semantic_role": "chassis", "required_for_runtime": True, "source_binding": {"source_path": FIXTURE, "object_ids": ["cube"], "primitive_ids": []}, "deform_region_ids": [], "material_region_ids": []}],
        "frames": [],
        "stations": [], "deform_regions": [], "parameters": [], "material_regions": [], "material_recipes": [], "livery_layers": [],
        "view_definitions": [],
        "validation_policy": {"topology_changes_allowed": False, "symmetry_default": True, "ground_contact_required": True, "uv_preservation_required": True, "source_mutation_allowed": False, "tolerance_m": 0.000001},
    }
    required = ["vehicle_origin", "front_axle_center", "rear_axle_center", "wheel_fl", "wheel_fr", "wheel_rl", "wheel_rr", "front_wing_mount", "rear_wing_mount"]
    for index, role in enumerate(required):
        data["frames"].append({"frame_id": f"frame-{role.replace('_','-')}", "semantic_role": role, "parent_frame_id": None, "transform": {"translation_m": [0, 0, index], "rotation_xyzw": [0, 0, 0, 1], "scale": [1, 1, 1]}})
    data["frames"][1]["transform"]["translation_m"] = [0, 0, -1]
    return VehicleDocument.from_dict(data)


class CadViewTests(unittest.TestCase):
    def test_four_semantic_views_are_byte_deterministic(self):
        first, manifest_a = compile_cad_views(document(), repo_root=REPO)
        second, manifest_b = compile_cad_views(document(), repo_root=REPO)
        self.assertEqual(first, second)
        self.assertEqual(cad_manifest_bytes(manifest_a), cad_manifest_bytes(manifest_b))
        self.assertEqual(set(first), {"top.svg", "side.svg", "front.svg", "rear.svg"})

    def test_svg_carries_persistent_ids_and_hashes(self):
        views, manifest = compile_cad_views(document(), repo_root=REPO)
        top = views["top.svg"].decode("utf-8")
        self.assertIn('data-schema="vehicle-cad-svg/v1"', top)
        self.assertIn('id="component-cube"', top)
        self.assertIn('data-document-sha256=', top)
        self.assertGreater(manifest["views"][0]["edge_count"], 0)
        self.assertEqual(manifest["views"][0]["sha256"], sha256(views["top.svg"]).hexdigest().upper())


if __name__ == "__main__":
    unittest.main()
