from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TOOLS = ROOT / "tools"
sys.path.insert(0, str(TOOLS))

from semantic_svg import SemanticSvgError, load_palette, pigment, serialize_svg, validate_svg  # noqa: E402
from render_semantic_cards import render_one  # noqa: E402
from generate_semantic_tree import TreeRecipe, generate_tree_svg  # noqa: E402
from voxelize_semantic_tree import extract_leaf_sprites, voxelize  # noqa: E402


class SemanticCardTests(unittest.TestCase):
    def test_examples_validate_and_cover_all_categories(self) -> None:
        analyses = [validate_svg(path)[1] for path in sorted((ROOT / "examples").glob("*.svg"))]
        self.assertGreaterEqual(len(analyses), 9)
        self.assertEqual({"tree", "bush", "grass"}, {item["kind"] for item in analyses})
        self.assertTrue(all(item["drawables"] >= 3 for item in analyses))
        self.assertTrue(all(item["drawables"] >= 25 for item in analyses if item["kind"] == "tree"))

    def test_external_resources_are_rejected(self) -> None:
        malicious = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"
          data-schema-version="1" data-asset-id="bad" data-asset-kind="tree">
          <image href="https://example.com/a.png"/>
        </svg>"""
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "bad.svg"
            path.write_text(malicious, encoding="utf-8")
            with self.assertRaises(SemanticSvgError):
                validate_svg(path)

    def test_pigmentation_changes_paint_not_path_geometry(self) -> None:
        source = ROOT / "examples" / "tree_coastal_01.svg"
        root, analysis = validate_svg(source)
        palette_path = ROOT / "palettes" / "south_america_west_low.json"
        palette = load_palette(palette_path)
        colored, metrics = pigment(root, palette, analysis["kind"])
        output = serialize_svg(colored).decode("utf-8")
        expected_base = json.loads(palette_path.read_text(encoding="utf-8"))["categories"]["tree"]["foliage"][2]
        self.assertIn(expected_base, output)
        self.assertIn("data-pigmented-biome=\"south_america_west_low\"", output)
        self.assertGreater(len(metrics["mapping"]), 3)
        self.assertEqual(source.read_text(encoding="utf-8").count(" d=\""), output.count(" d=\""))

    @unittest.skipUnless(importlib.util.find_spec("resvg_py"), "resvg_py is not installed")
    def test_render_is_byte_deterministic(self) -> None:
        source = ROOT / "examples" / "bush_coastal_01.svg"
        palette = ROOT / "palettes" / "south_america_west_low.json"
        with tempfile.TemporaryDirectory() as first, tempfile.TemporaryDirectory() as second:
            a = render_one(source, palette, Path(first))
            b = render_one(source, palette, Path(second))
            self.assertEqual(a["png_sha256"], b["png_sha256"])
            self.assertEqual(a["alpha_bbox"], b["alpha_bbox"])
            self.assertGreater(a["visible_pixels"], 10000)
            self.assertEqual("arboreal_stipple_v1", a["art_style"]["id"])
            self.assertGreater(a["art_style"]["transparent_leaf_gaps"], 100)
            self.assertGreater(a["art_style"]["pigmented_wood_pixels"], 0)

    def test_recursive_generator_is_deterministic_and_detailed(self) -> None:
        recipe = TreeRecipe("generated_test", seed=73, depth=7, leaf_density=2.2)
        first = generate_tree_svg(recipe)
        second = generate_tree_svg(recipe)
        self.assertEqual(first, second)
        self.assertGreater(first.count("<path"), 150)
        self.assertGreater(first.count("<ellipse"), 200)
        self.assertIn('fill="none" stroke="currentColor"', first)
        self.assertEqual(6, first.count('data-role="root"'))

    def test_semantic_voxels_have_depth_and_materials(self) -> None:
        source = ROOT / "examples" / "tree_recursive_01.svg"
        first, report = voxelize(source, (64, 64, 32))
        second, _ = voxelize(source, (64, 64, 32))
        self.assertTrue((first == second).all())
        self.assertGreater(report["occupied_voxels"], 1000)
        self.assertGreater(len(set(first[first > 0].tolist())), 2)
        occupied_depth = first.any(axis=(0, 1)).nonzero()[0]
        self.assertGreater(int(occupied_depth[-1] - occupied_depth[0]), 10)

    def test_hybrid_mode_keeps_leaves_out_of_voxel_volume(self) -> None:
        source = ROOT / "examples" / "tree_recursive_01.svg"
        wood, report = voxelize(source, (64, 64, 32), include_foliage=False)
        leaves = extract_leaf_sprites(source, (64, 64, 32))
        self.assertNotIn(4, set(wood[wood > 0].tolist()))
        self.assertEqual(696, len(leaves))
        self.assertFalse(report["foliage_voxelized"])


if __name__ == "__main__":
    unittest.main()
