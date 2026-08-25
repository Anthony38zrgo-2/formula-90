from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[4]
TOOLS = ROOT / "game" / "resources" / "environment" / "tools"
sys.path.insert(0, str(TOOLS))

from audit_vegetation_3d_library import audit_manifest, inspect_glb  # noqa: E402
from build_vegetation_3d_library import build_asset  # noqa: E402


class Vegetation3dLibraryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.recipe_doc = json.loads((ROOT / "game/resources/environment/recipes/vegetation_library.json").read_text(encoding="utf-8"))
        self.palette_doc = json.loads((ROOT / self.recipe_doc["palette"]).read_text(encoding="utf-8"))

    def test_committed_library_passes_independent_glb_audit(self) -> None:
        report = audit_manifest(ROOT, ROOT / "game/resources/environment/assets/manifest.json")
        self.assertEqual(report["failures"], [])
        self.assertEqual(report["asset_count"], 12)

    def test_recipes_define_six_distinct_families_per_kind(self) -> None:
        for kind in ("tree", "bush"):
            recipes = [item for item in self.recipe_doc["assets"] if item["kind"] == kind]
            self.assertEqual(len(recipes), 6)
            self.assertEqual(len({item["family"] for item in recipes}), 6)

    def test_tree_and_bush_prototypes_are_byte_deterministic(self) -> None:
        for asset_id in ("tree_3d_01", "bush_3d_01"):
            recipe = next(item for item in self.recipe_doc["assets"] if item["id"] == asset_id)
            palette = self.palette_doc["categories"][recipe["kind"]]
            hashes = []
            with tempfile.TemporaryDirectory() as temporary:
                base = Path(temporary)
                for run in ("a", "b"):
                    repo = base / run
                    output = repo / "game/resources/environment/assets"
                    audit = repo / "game/resources/environment/generated/library_audit"
                    result = build_asset(recipe, palette, output, audit)
                    hashes.append(hashlib.sha256((repo / result["glb"]).read_bytes()).hexdigest())
            self.assertEqual(hashes[0], hashes[1], asset_id)

    def test_all_assets_have_real_bounds_colors_and_double_sided_material(self) -> None:
        manifest = json.loads((ROOT / "game/resources/environment/assets/manifest.json").read_text(encoding="utf-8"))
        for asset in manifest["assets"]:
            report = inspect_glb(ROOT / asset["glb"])
            self.assertEqual(report["roles"], ["foliage", "wood"], asset["id"])
            self.assertEqual(report["colored_roles"], ["foliage", "wood"], asset["id"])
            self.assertTrue(asset["foliage_double_sided_material"], asset["id"])
            for key in ("width", "height", "depth"):
                self.assertAlmostEqual(report["dimensions_m"][key], asset["dimensions_m"][key], places=3)


if __name__ == "__main__":
    unittest.main()
