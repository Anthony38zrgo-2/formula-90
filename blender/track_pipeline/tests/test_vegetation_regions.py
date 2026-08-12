import unittest
from pathlib import Path
import sys
import tempfile

import cv2
import numpy as np

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "blender" / "track_pipeline"))

from semantic_layout_common import WorldRasterTransform
from vegetation_regions import (
    Region,
    assign_instances_to_regions,
    derive_regions,
    sample_new_placements,
    scale_boundary,
    translate_boundary,
)
from vegetation_region_ops import apply_region_operation


def _dummy_transform(width=200, height=200, bounds=(-100.0, -100.0, 100.0, 100.0)):
    return WorldRasterTransform(width, height, *bounds)


def _make_semantic(polygons, width=200, height=200):
    """Render one category mask from a list of polygon pixel paths."""
    img = np.zeros((height, width, 3), dtype=np.uint8)
    palette = (10, 200, 10)
    for path in polygons:
        pts = np.array(path, dtype=np.int32).reshape(-1, 1, 2)
        cv2.fillPoly(img, [pts], palette)
    return img, palette


class RegionDerivationTests(unittest.TestCase):
    def test_derive_regions_two_components(self):
        transform = _dummy_transform()
        semantic, palette = _make_semantic([
            [(20, 20), (60, 20), (60, 60), (20, 60)],
            [(120, 120), (160, 120), (160, 160), (120, 160)],
        ])
        regions, labels, label_map = derive_regions(
            semantic, palette, "grass",
            transform=transform, seed=1, spacing_m=4.0, target_count=10,
            region_prefix="grass", min_area_px=8,
        )
        self.assertEqual(len(regions), 2)
        self.assertEqual({r.region_id for r in regions}, {"grass_000", "grass_001"})
        # deterministic: two runs equal
        regions2, labels2, _ = derive_regions(
            semantic, palette, "grass",
            transform=transform, seed=1, spacing_m=4.0, target_count=10,
            region_prefix="grass", min_area_px=8,
        )
        self.assertEqual([r.region_id for r in regions], [r.region_id for r in regions2])

    def test_assign_instances_by_pixel_label_concave(self):
        # A U-shaped region: instances in the concave notch must still belong to it.
        transform = _dummy_transform()
        # U shape (pixel coords): outer rectangle with a notch
        img = np.zeros((200, 200, 3), dtype=np.uint8)
        palette = (10, 200, 10)
        cv2.rectangle(img, (20, 20), (160, 160), palette, -1)
        cv2.rectangle(img, (50, 50), (130, 120), (0, 0, 0), -1)  # notch (hollow)
        regions, labels, label_map = derive_regions(
            img, palette, "grass",
            transform=transform, seed=1, spacing_m=4.0, target_count=10,
            region_prefix="grass", min_area_px=8,
        )
        # A world point inside the notch (world -> pixel inside hollow region)
        veg = [{"instance_id": "g0", "position_xz": [40.0, 40.0]}]  # pixel (40,160)-> world x=40? see transform
        assignment = assign_instances_to_regions(
            veg, regions, "grass",
            transform=transform, labels=labels, label_of_region=label_map,
        )
        # world_to_pixel: x -> px; pixel (40,160) lies inside outer rect, outside notch
        self.assertEqual(assignment["g0"], regions[0].region_id)

    def test_translate_and_scale_boundary(self):
        boundary = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]
        moved = translate_boundary(boundary, 5.0, -3.0)
        self.assertEqual(moved[0], [5.0, -3.0])
        scaled = scale_boundary(boundary, 2.0, [0.0, 0.0])
        self.assertEqual(scaled[2], [20.0, 20.0])


class RegionSampleTests(unittest.TestCase):
    def test_sample_new_placements_only_in_new_area(self):
        old_polygon = [[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]]
        new_polygon = [[-20.0, -20.0], [60.0, -20.0], [60.0, 60.0], [-20.0, 60.0]]
        region = Region("grass_000", "grass", new_polygon, seed=7, spacing_m=4.0, target_count=50)
        placements = sample_new_placements(
            new_polygon=new_polygon,
            old_polygon=old_polygon,
            existing=[],
            region=region,
            asset_pool=["grass_v2_01"],
            seed=7,
            spacing_m=4.0,
        )
        self.assertGreater(len(placements), 0)
        # None should fall inside the old polygon (the added ring only)
        for p in placements:
            x, z = p["position_xz"]
            inside_old = -1e-9 <= x <= 40.0 + 1e-9 and -1e-9 <= z <= 40.0 + 1e-9
            self.assertFalse(inside_old, (x, z))

    def test_sample_new_preserves_existing_and_deterministic(self):
        old_polygon = [[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]]
        new_polygon = [[-20.0, -20.0], [60.0, -20.0], [60.0, 60.0], [-20.0, 60.0]]
        region = Region("grass_000", "grass", new_polygon, seed=7, spacing_m=4.0, target_count=50)
        existing = [{"position_xz": [-10.0, -10.0]}]
        a = sample_new_placements(
            new_polygon=new_polygon, old_polygon=old_polygon, existing=existing,
            region=region, asset_pool=["grass_v2_01"], seed=7, spacing_m=4.0,
            min_separation_m=1.0,
        )
        b = sample_new_placements(
            new_polygon=new_polygon, old_polygon=old_polygon, existing=existing,
            region=region, asset_pool=["grass_v2_01"], seed=7, spacing_m=4.0,
            min_separation_m=1.0,
        )
        self.assertEqual(a, b)
        for p in a:
            self.assertGreaterEqual(
                np.hypot(p["position_xz"][0] + 10, p["position_xz"][1] + 10), 1.0 - 1e-9
            )


class RegionOpsTests(unittest.TestCase):
    def _canonical(self):
        from xml.etree import ElementTree as ET

        ns = "http://www.w3.org/2000/svg"
        root = ET.Element(f"{{{ns}}}svg")
        root.set("viewBox", "0 0 100 100")
        root.set("data-track-id", "t")
        g = ET.SubElement(root, f"{{{ns}}}g")
        g.set("data-role", "vegetation-region")
        g.set("data-region-id", "grass_000")
        g.set("data-category", "grass")
        g.set("data-seed", "7")
        g.set("data-spacing-m", "4.0")
        g.set("data-target-count", "2")
        poly = ET.SubElement(g, f"{{{ns}}}polygon")
        poly.set("data-role", "vegetation-boundary")
        poly.set("points", "0,0 10,0 10,10 0,10")
        for i, (cx, cy) in enumerate([(2.0, 2.0), (8.0, 8.0)]):
            c = ET.SubElement(g, f"{{{ns}}}circle")
            c.set("data-role", "asset-instance")
            c.set("data-instance-id", f"g{i}")
            c.set("data-asset-id", "grass_v2_01")
            c.set("data-kind", "vegetation")
            c.set("cx", str(cx))
            c.set("cy", str(cy))
            c.set("r", "0.4")
            c.set("data-generated-by-region", "grass_000")
        return root

    def test_move_region_translates_members_and_boundary(self):
        root = self._canonical()
        apply_region_operation(root, "move", "grass_000", {"dx": 5.0, "dz": -3.0})
        g = root.find(".//{*}g[@data-region-id='grass_000']")
        polys = [e for e in g if e.get("data-role") == "vegetation-boundary"]
        self.assertEqual(polys[0].get("points").split()[0], "5.0000,-3.0000")
        circles = [e for e in g if e.get("data-role") == "asset-instance"]
        self.assertEqual(float(circles[0].get("cx")), 7.0)
        self.assertEqual(float(circles[0].get("cy")), -1.0)

    def test_scale_region_around_anchor(self):
        root = self._canonical()
        apply_region_operation(root, "scale", "grass_000", {"factor": 2.0, "anchor": [0.0, 0.0]})
        g = root.find(".//{*}g[@data-region-id='grass_000']")
        circles = [e for e in g if e.get("data-role") == "asset-instance"]
        self.assertEqual(float(circles[1].get("cx")), 16.0)
        self.assertEqual(float(circles[1].get("cy")), 16.0)

    def test_extend_region_preserves_old_and_adds_new(self):
        root = self._canonical()
        new_boundary = [[-10.0, -10.0], [20.0, -10.0], [20.0, 20.0], [-10.0, 20.0]]
        before = [e.get("data-instance-id") for e in root.iter() if e.get("data-role") == "asset-instance"]
        apply_region_operation(
            root, "extend", "grass_000",
            {"new_boundary": new_boundary, "asset_pool": ["grass_v2_01"]},
        )
        after = [e.get("data-instance-id") for e in root.iter() if e.get("data-role") == "asset-instance"]
        self.assertTrue(set(before).issubset(set(after)))
        self.assertGreater(len(after), len(before))
        # new instances belong to the region
        for eid in after:
            if eid not in before:
                circles = [e for e in root.iter() if e.get("data-instance-id") == eid]
                self.assertEqual(circles[0].get("data-generated-by-region"), "grass_000")

    def test_unknown_operation_rejected(self):
        root = self._canonical()
        with self.assertRaises(ValueError):
            apply_region_operation(root, "teleport", "grass_000", {})


def _rmtree(path: Path):
    if path.exists():
        for child in path.iterdir():
            if child.is_dir():
                _rmtree(child)
            else:
                child.unlink()
        path.rmdir()


if __name__ == "__main__":
    unittest.main()
