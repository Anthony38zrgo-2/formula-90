from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import numpy as np
from PIL import Image

from generate_procedural_textures import apply_ground_cover_detail
from apply_ground_cover_to_terrain import load_terrain_base
from texture_forge import make_seamless_edges


class GroundCoverTextureTests(unittest.TestCase):
    def test_terrain_edges_are_exactly_seamless_without_mirroring_the_interior(self) -> None:
        pixels = np.arange(8 * 8 * 3, dtype=np.uint8).reshape((8, 8, 3))

        result = np.asarray(make_seamless_edges(Image.fromarray(pixels, "RGB"), 0.25))

        self.assertTrue(np.array_equal(result[:, 0], result[:, -1]))
        self.assertTrue(np.array_equal(result[0], result[-1]))
        self.assertFalse(np.array_equal(result[:, 3], result[:, 4]))
        self.assertFalse(np.array_equal(result[3], result[4]))

    def test_curated_base_is_resized_to_configured_runtime_resolution(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            source = Path(temp) / "terrain.png"
            Image.new("RGB", (32, 32), (90, 80, 65)).save(source)

            result = load_terrain_base(source, 128)

            self.assertEqual(result.size, (128, 128))

    def test_hybrid_ground_cover_is_deterministic_and_partial(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source_a = root / "a.png"
            source_b = root / "b.png"
            Image.new("RGB", (16, 16), (30, 120, 40)).save(source_a)
            Image.new("RGB", (16, 16), (130, 110, 60)).save(source_b)
            base = Image.new("RGB", (64, 64), (90, 80, 65))

            first = apply_ground_cover_detail(base, [source_a, source_b], 1995, "test", 0.5, 0.35)
            second = apply_ground_cover_detail(base, [source_a, source_b], 1995, "test", 0.5, 0.35)
            first_pixels = np.asarray(first)

            self.assertTrue(np.array_equal(first_pixels, np.asarray(second)))
            changed = np.any(first_pixels != np.asarray(base), axis=-1)
            self.assertGreater(float(changed.mean()), 0.40)
            self.assertLess(float(changed.mean()), 0.60)


if __name__ == "__main__":
    unittest.main()
