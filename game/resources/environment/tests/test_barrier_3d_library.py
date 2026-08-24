from __future__ import annotations

import copy
import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path

import numpy as np
import trimesh

ROOT = Path(__file__).resolve().parents[4]
TOOLS = ROOT / "game/resources/environment/tools"
sys.path.insert(0, str(TOOLS))

from build_barrier_3d_library import (  # noqa: E402
    _hex,
    build_asset,
    validate_recipe_document,
)


class Barrier3dLibraryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.recipe_path = ROOT / "game/resources/environment/recipes/barrier_library.json"
        cls.recipe_doc = json.loads(cls.recipe_path.read_text(encoding="utf-8"))
        cls.palette_path = ROOT / cls.recipe_doc["palette"]
        cls.palette_doc = json.loads(cls.palette_path.read_text(encoding="utf-8"))
        cls.contract_path = ROOT / cls.recipe_doc["construction_manifest"]
        cls.contract_doc = json.loads(cls.contract_path.read_text(encoding="utf-8"))
        cls.manifest_path = ROOT / "game/resources/environment/assets/barriers/barrier_manifest.json"
        cls.manifest = json.loads(cls.manifest_path.read_text(encoding="utf-8"))

    def test_hex_parser_uses_exact_rgb_channels(self) -> None:
        self.assertEqual(_hex("#1A1C22"), (0x1A, 0x1C, 0x22))
        self.assertEqual(_hex("#F8F8F4"), (0xF8, 0xF8, 0xF4))
        self.assertEqual(_hex("#C22018"), (0xC2, 0x20, 0x18))
        with self.assertRaises(ValueError):
            _hex("#fff")

    def test_recipe_contract_rejects_duplicates_and_bad_dimensions(self) -> None:
        validate_recipe_document(self.recipe_doc, self.palette_doc, self.contract_doc)
        duplicate = copy.deepcopy(self.recipe_doc)
        duplicate["assets"].append(copy.deepcopy(duplicate["assets"][0]))
        with self.assertRaisesRegex(ValueError, "Duplicate"):
            validate_recipe_document(duplicate, self.palette_doc, self.contract_doc)
        too_tall = copy.deepcopy(self.recipe_doc)
        too_tall["assets"][0]["height_m"] = 3.75
        with self.assertRaisesRegex(ValueError, "implausible"):
            validate_recipe_document(too_tall, self.palette_doc, self.contract_doc)

    def test_manifest_schema_sources_and_asset_count(self) -> None:
        self.assertEqual(self.manifest["schema_version"], 2)
        self.assertEqual(self.manifest["generator"], "procedural_barrier_v2")
        self.assertEqual(self.manifest["shading_contract"], "procedural_vertex_color_v1")
        self.assertEqual(len(self.manifest["assets"]), len(self.recipe_doc["assets"]))
        self.assertEqual(
            self.manifest["recipe_document_sha256"],
            hashlib.sha256(self.recipe_path.read_bytes()).hexdigest(),
        )
        self.assertEqual(
            self.manifest["palette_sha256"],
            hashlib.sha256(self.palette_path.read_bytes()).hexdigest(),
        )
        self.assertEqual(self.manifest["construction_manifest"], self.recipe_doc["construction_manifest"])
        self.assertEqual(
            self.manifest["construction_manifest_sha256"],
            hashlib.sha256(self.contract_path.read_bytes()).hexdigest(),
        )
        catalog = ROOT / self.manifest["human_gate_catalog"]
        self.assertTrue(catalog.is_file())
        self.assertEqual(hashlib.sha256(catalog.read_bytes()).hexdigest(),
                         self.manifest["human_gate_catalog_sha256"])

    def test_all_outputs_are_deterministic(self) -> None:
        recipe = self.recipe_doc["assets"][0]
        hashes = []
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            for run in ("a", "b"):
                output = base / run / "assets"
                result = build_asset(recipe, self.palette_doc, output, base / run / "review", base / run)
                hashes.append((result["visual_sha256"], result["collision_sha256"], result["strip_sha256"]))
        self.assertEqual(hashes[0], hashes[1])

    def test_manifest_files_hashes_and_geometry_contract(self) -> None:
        recipe_by_id = {recipe["id"]: recipe for recipe in self.recipe_doc["assets"]}
        for asset in self.manifest["assets"]:
            recipe = recipe_by_id[asset["id"]]
            for path_key, hash_key in (
                ("visual_glb", "visual_sha256"),
                ("collision_glb", "collision_sha256"),
                ("strip_glb", "strip_sha256"),
            ):
                path = ROOT / asset[path_key]
                self.assertTrue(path.is_file(), f"Missing {path_key} for {asset['id']}")
                self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), asset[hash_key])

            visual = next(iter(trimesh.load(ROOT / asset["visual_glb"]).geometry.values()))
            collision = next(iter(trimesh.load(ROOT / asset["collision_glb"]).geometry.values()))
            strip = next(iter(trimesh.load(ROOT / asset["strip_glb"]).geometry.values()))
            for mesh in (visual, collision, strip):
                self.assertTrue(mesh.is_watertight, asset["id"])
                self.assertTrue(mesh.is_winding_consistent, asset["id"])
                self.assertTrue(np.isfinite(mesh.vertices).all(), asset["id"])
                self.assertTrue(np.all(mesh.area_faces > 1e-9), asset["id"])

            half_width = float(recipe["width_m"]) * 0.5
            self.assertGreaterEqual(float(visual.bounds[0][0]), -half_width - 1e-4)
            self.assertLessEqual(float(visual.bounds[1][0]), half_width + 1e-4)
            self.assertGreaterEqual(float(visual.extents[0]), float(recipe["width_m"]) * 0.90)
            self.assertAlmostEqual(float(visual.bounds[0][1]), 0.0, places=4)
            self.assertLessEqual(len(visual.faces), int(recipe["triangle_budget"]))
            self.assertEqual(len(strip.faces), len(visual.faces) * 5)
            self.assertEqual(len(collision.faces), 12)
            self.assertGreater(len(visual.visual.vertex_colors), 0)
            unique_colours = np.unique(visual.visual.vertex_colors[:, :3], axis=0)
            self.assertGreater(len(unique_colours), 2, f"No procedural shading in {asset['id']}")
            self.assertEqual(asset["shading"]["mode"], "procedural_vertex_color_v1")
            self.assertFalse(asset["shading"]["cast_shadows_baked"])
            self.assertLessEqual(float(visual.bounds[1][1]), 1.50)

            self.assertTrue((ROOT / asset["preview"]).is_file())
            self.assertTrue((ROOT / asset["strip_preview"]).is_file())

    def test_tire_depth_variants_do_not_multiply_height(self) -> None:
        tire_assets = [asset for asset in self.manifest["assets"] if asset["family"] in {"tire_black", "tire_navy_white"}]
        for asset in tire_assets:
            visual = next(iter(trimesh.load(ROOT / asset["visual_glb"]).geometry.values()))
            self.assertLessEqual(float(visual.bounds[1][1]), 1.25 + 1e-4)
            if asset["depth_layers"] > 1:
                self.assertGreater(float(visual.extents[2]), 0.62)

    def test_tires_are_stacked_flat_with_vertical_axles(self) -> None:
        recipe_by_id = {recipe["id"]: recipe for recipe in self.recipe_doc["assets"]}
        for asset in self.manifest["assets"]:
            if asset["family"] not in {"tire_black", "tire_navy_white"}:
                continue
            recipe = recipe_by_id[asset["id"]]
            visual = next(iter(trimesh.load(ROOT / asset["visual_glb"]).geometry.values()))
            components = visual.split(only_watertight=False)
            expected_front = int(recipe["columns_per_module"]) * int(recipe["tires_per_column"])
            self.assertGreaterEqual(len(components), expected_front)
            expected_tire_triangles = (2 * int(recipe["tire_major_sections"]) *
                                       int(recipe["tire_minor_sections"]))
            for tire in components:
                # A flat tyre has a narrow vertical Y extent and a full diameter
                # in both horizontal X/Z axes.
                self.assertAlmostEqual(float(tire.extents[1]), float(recipe["tire_width_m"]), places=4)
                self.assertAlmostEqual(float(tire.extents[0]), float(recipe["tire_outer_diameter_m"]), places=4)
                self.assertAlmostEqual(float(tire.extents[2]), float(recipe["tire_outer_diameter_m"]), places=4)
                self.assertEqual(len(tire.faces), expected_tire_triangles)


if __name__ == "__main__":
    unittest.main()
