from __future__ import annotations

import sys
import unittest
from pathlib import Path

import cv2
import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from analyze_vegetation_textures import visible_magenta_mask
from vegetation_texture_common import MAGENTA_RGB, prepare_vegetation_card_rgba


class VegetationTexturePipelineTests(unittest.TestCase):
    def test_keys_magenta_and_interior_holes_but_preserves_autumn_pink(self):
        img = np.empty((256, 256, 4), dtype=np.uint8)
        img[..., :3] = MAGENTA_RGB
        img[..., 3] = 255
        img[30:236, 55:201, :3] = (42, 118, 54)
        img[90:132, 95:155, :3] = MAGENTA_RGB
        img[150:190, 80:125, :3] = (220, 82, 168)

        out, metrics = prepare_vegetation_card_rgba(img, (128, 128), MAGENTA_RGB, 1)
        self.assertEqual(metrics["bottom_gap_px"], 0)
        self.assertEqual(int(out[0, 0, 3]), 0)
        self.assertLess(int(out[55, 62, 3]), 20)
        self.assertGreater(int(out[85, 51, 3]), 220)

    def test_premultiplied_resize_does_not_create_magenta_fringe(self):
        size = 384
        yy, xx = np.mgrid[0:size, 0:size]
        dist = np.sqrt((xx - size * .5) ** 2 + (yy - size * .52) ** 2)
        fg = np.array([35.0, 115.0, 55.0])
        bg = MAGENTA_RGB.astype(float)
        object_alpha = np.clip((145.0 - dist) / 8.0, 0.0, 1.0)
        rgb = fg[None, None, :] * object_alpha[..., None] + bg[None, None, :] * (1 - object_alpha[..., None])
        img = np.empty((size, size, 4), dtype=np.uint8)
        img[..., :3] = np.rint(rgb).astype(np.uint8)
        img[..., 3] = 255

        out, _ = prepare_vegetation_card_rgba(img, (128, 128), MAGENTA_RGB, 1)
        self.assertEqual(int(visible_magenta_mask(out).sum()), 0)

    def test_real_alpha_card_does_not_receive_magenta(self):
        img = np.zeros((192, 192, 4), dtype=np.uint8)
        cv2.rectangle(img, (50, 30), (142, 180), (210, 75, 165, 255), thickness=-1)
        out, metrics = prepare_vegetation_card_rgba(img, (128, 128), background_rgb=None)

        self.assertEqual(metrics["bottom_gap_px"], 0)
        transparent = out[..., 3] == 0
        exact_magenta = transparent & np.all(out[..., :3] == MAGENTA_RGB, axis=-1)
        self.assertEqual(int(exact_magenta.sum()), 0)
        self.assertGreater(int(out[..., 3].max()), 240)

    def test_processing_is_bit_deterministic_in_same_environment(self):
        rng = np.random.default_rng(1995)
        img = np.empty((320, 320, 4), dtype=np.uint8)
        img[..., :3] = MAGENTA_RGB
        img[..., 3] = 255
        img[45:300, 75:245, :3] = rng.integers(20, 150, (255, 170, 3), dtype=np.uint8)

        a, metrics_a = prepare_vegetation_card_rgba(img, (128, 128), MAGENTA_RGB, 1)
        b, metrics_b = prepare_vegetation_card_rgba(img, (128, 128), MAGENTA_RGB, 1)
        self.assertTrue(np.array_equal(a, b))
        self.assertEqual(metrics_a, metrics_b)

    def test_validator_flags_visible_key(self):
        rgba = np.zeros((8, 8, 4), dtype=np.uint8)
        rgba[2:6, 2:6, :3] = MAGENTA_RGB
        rgba[2:6, 2:6, 3] = 255
        self.assertEqual(int(visible_magenta_mask(rgba).sum()), 16)

    def test_validator_preserves_autumn_pink(self):
        rgba = np.zeros((8, 8, 4), dtype=np.uint8)
        rgba[2:6, 2:6, :3] = (220, 82, 168)
        rgba[2:6, 2:6, 3] = 255
        self.assertEqual(int(visible_magenta_mask(rgba).sum()), 0)

    def test_source_keyer_output_passes_independent_gate(self):
        img = np.empty((256, 256, 4), dtype=np.uint8)
        img[..., :3] = MAGENTA_RGB
        img[..., 3] = 255
        img[30:235, 55:200, :3] = (40, 118, 52)
        output, metrics = prepare_vegetation_card_rgba(img, (128, 128), MAGENTA_RGB, 1)
        self.assertEqual(metrics["bottom_gap_px"], 0)
        self.assertEqual(int(visible_magenta_mask(output).sum()), 0)


if __name__ == "__main__":
    unittest.main()
