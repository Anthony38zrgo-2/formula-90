from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

import cv2
import numpy as np
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from analyze_vegetation_textures import visible_key_mask
from recut_vegetation_textures import recut_image
from migrate_vegetation_source_keys import rekey_source_rgba
from vegetation_texture_common import (
    DEFAULT_BACKGROUND_RGB,
    LEGACY_MAGENTA_RGB,
    prepare_vegetation_card_rgba,
    remove_isolated_key_speckles_rgba,
)


class VegetationTexturePipelineTests(unittest.TestCase):
    def test_keys_cyan_and_interior_holes_but_preserves_autumn_pink(self):
        img = np.empty((256, 256, 4), dtype=np.uint8)
        img[..., :3] = DEFAULT_BACKGROUND_RGB
        img[..., 3] = 255
        img[30:236, 55:201, :3] = (42, 118, 54)
        img[90:132, 95:155, :3] = DEFAULT_BACKGROUND_RGB
        img[150:190, 80:125, :3] = (220, 82, 168)
        out, metrics = prepare_vegetation_card_rgba(img, (128, 128), DEFAULT_BACKGROUND_RGB, 1)
        self.assertEqual(metrics["bottom_gap_px"], 0)
        self.assertEqual(int(out[0, 0, 3]), 0)
        self.assertLess(int(out[55, 62, 3]), 20)
        self.assertGreater(int(out[85, 51, 3]), 220)

    def test_premultiplied_resize_does_not_create_cyan_fringe(self):
        size = 384
        yy, xx = np.mgrid[0:size, 0:size]
        dist = np.sqrt((xx - size * .5) ** 2 + (yy - size * .52) ** 2)
        fg = np.array([35.0, 115.0, 55.0])
        bg = DEFAULT_BACKGROUND_RGB.astype(float)
        object_alpha = np.clip((145.0 - dist) / 8.0, 0.0, 1.0)
        rgb = fg[None, None, :] * object_alpha[..., None] + bg[None, None, :] * (1 - object_alpha[..., None])
        img = np.empty((size, size, 4), dtype=np.uint8)
        img[..., :3] = np.rint(rgb).astype(np.uint8)
        img[..., 3] = 255
        out, _ = prepare_vegetation_card_rgba(img, (128, 128), DEFAULT_BACKGROUND_RGB, 1)
        self.assertEqual(int(visible_key_mask(out, DEFAULT_BACKGROUND_RGB).sum()), 0)

    def test_real_alpha_card_does_not_receive_cyan(self):
        img = np.zeros((192, 192, 4), dtype=np.uint8)
        cv2.rectangle(img, (50, 30), (142, 180), (210, 75, 165, 255), thickness=-1)
        out, metrics = prepare_vegetation_card_rgba(img, (128, 128), background_rgb=None)
        self.assertEqual(metrics["bottom_gap_px"], 0)
        transparent = out[..., 3] == 0
        exact_cyan = transparent & np.all(out[..., :3] == DEFAULT_BACKGROUND_RGB, axis=-1)
        self.assertEqual(int(exact_cyan.sum()), 0)
        self.assertGreater(int(out[..., 3].max()), 240)

    def test_processing_is_bit_deterministic_in_same_environment(self):
        rng = np.random.default_rng(1995)
        img = np.empty((320, 320, 4), dtype=np.uint8)
        img[..., :3] = DEFAULT_BACKGROUND_RGB
        img[..., 3] = 255
        img[45:300, 75:245, :3] = rng.integers(20, 150, (255, 170, 3), dtype=np.uint8)
        a, metrics_a = prepare_vegetation_card_rgba(img, (128, 128), DEFAULT_BACKGROUND_RGB, 1)
        b, metrics_b = prepare_vegetation_card_rgba(img, (128, 128), DEFAULT_BACKGROUND_RGB, 1)
        self.assertTrue(np.array_equal(a, b))
        self.assertEqual(metrics_a, metrics_b)

    def test_validator_flags_visible_cyan_key(self):
        rgba = np.zeros((8, 8, 4), dtype=np.uint8)
        rgba[2:6, 2:6, :3] = DEFAULT_BACKGROUND_RGB
        rgba[2:6, 2:6, 3] = 255
        self.assertEqual(int(visible_key_mask(rgba, DEFAULT_BACKGROUND_RGB).sum()), 16)

    def test_validator_preserves_autumn_pink(self):
        rgba = np.zeros((8, 8, 4), dtype=np.uint8)
        rgba[2:6, 2:6, :3] = (220, 82, 168)
        rgba[2:6, 2:6, 3] = 255
        self.assertEqual(int(visible_key_mask(rgba, DEFAULT_BACKGROUND_RGB).sum()), 0)

    def test_source_keyer_output_passes_independent_gate(self):
        img = np.empty((256, 256, 4), dtype=np.uint8)
        img[..., :3] = DEFAULT_BACKGROUND_RGB
        img[..., 3] = 255
        img[30:235, 55:200, :3] = (40, 118, 52)
        output, metrics = prepare_vegetation_card_rgba(img, (128, 128), DEFAULT_BACKGROUND_RGB, 1)
        self.assertEqual(metrics["bottom_gap_px"], 0)
        self.assertEqual(int(visible_key_mask(output, DEFAULT_BACKGROUND_RGB).sum()), 0)

    def test_legacy_recut_cleans_existing_128px_magenta_card(self):
        rgba = np.empty((128, 128, 4), dtype=np.uint8)
        rgba[..., :3] = LEGACY_MAGENTA_RGB
        rgba[..., 3] = 255
        rgba[15:120, 28:100, :3] = (45, 116, 55)
        rgba[48:68, 52:76, :3] = LEGACY_MAGENTA_RGB
        with tempfile.TemporaryDirectory() as td:
            path = Path(td) / "south_america_west_low_tree_99.png"
            Image.fromarray(rgba, "RGBA").save(path)
            metrics = recut_image(path, pass_index=1)
            output = np.asarray(Image.open(path).convert("RGBA"), dtype=np.uint8)
        self.assertEqual(metrics["bottom_gap_px"], 0)
        self.assertEqual(int(visible_key_mask(output, LEGACY_MAGENTA_RGB).sum()), 0)
        self.assertEqual(int(output[0, 0, 3]), 0)
        self.assertLess(int(output[58, 64, 3]), 20)

    def test_legacy_speckle_cleanup_removes_isolated_magenta(self):
        rgba = np.zeros((32, 32, 4), dtype=np.uint8)
        rgba[4:28, 6:26, :3] = (70, 120, 60)
        rgba[4:28, 6:26, 3] = 255
        rgba[16, 16, :3] = LEGACY_MAGENTA_RGB
        rgba[16, 16, 3] = 255
        cleaned, metrics = remove_isolated_key_speckles_rgba(rgba, key_rgb=LEGACY_MAGENTA_RGB)
        self.assertGreaterEqual(metrics["removed_pixels"], 1)
        self.assertEqual(int(cleaned[16, 16, 3]), 0)

    def test_rekey_source_moves_background_to_cyan_and_preserves_pink(self):
        img = np.empty((128, 128, 4), dtype=np.uint8)
        img[..., :3] = LEGACY_MAGENTA_RGB
        img[..., 3] = 255
        img[20:120, 30:100, :3] = (45, 116, 55)
        img[55:70, 45:60, :3] = (220, 82, 168)
        migrated, metrics = rekey_source_rgba(img, LEGACY_MAGENTA_RGB, DEFAULT_BACKGROUND_RGB, 1)
        self.assertTrue(np.all(migrated[..., 3] == 255))
        self.assertTrue(np.all(migrated[0, 0, :3] == DEFAULT_BACKGROUND_RGB))
        self.assertGreater(int(migrated[60, 50, 0]), 180)
        self.assertEqual(metrics["source_key_hex"], "#FF00FF")
        self.assertEqual(metrics["target_key_hex"], "#00FFFF")


if __name__ == "__main__":
    unittest.main()
