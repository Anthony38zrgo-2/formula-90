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
from vegetation_texture_stylizer import stylize_card_rgba
from vegetation_texture_common import (
    DEFAULT_BACKGROUND_RGB,
    LEGACY_MAGENTA_RGB,
    prepare_vegetation_card_rgba,
    prepare_precut_card_preserve_aspect,
    remove_isolated_key_speckles_rgba,
)


class VegetationTexturePipelineTests(unittest.TestCase):
    def test_precut_aspect_fit_preserves_shape_and_bottom_anchor(self):
        rgba = np.zeros((100, 300, 4), dtype=np.uint8)
        rgba[10:90, 20:280, :3] = (185, 130, 55)
        rgba[10:90, 20:280, 3] = 255
        output, metrics = prepare_precut_card_preserve_aspect(rgba, (256, 256))
        self.assertEqual(metrics["bottom_gap_px"], 0)
        self.assertGreater(metrics["resized_visible_size"][0], metrics["resized_visible_size"][1] * 2)
        self.assertEqual(output.shape, (256, 256, 4))

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

    def test_anchor_validation_is_read_only(self):
        rgba = np.zeros((128, 128, 4), dtype=np.uint8)
        rgba[15:128, 28:100, :3] = (45, 116, 55)
        rgba[15:128, 28:100, 3] = 255
        with tempfile.TemporaryDirectory() as td:
            path = Path(td) / "south_america_west_low_tree_99.png"
            Image.fromarray(rgba, "RGBA").save(path)
            before = path.read_bytes()
            metrics = recut_image(path, pass_index=1)
            after = path.read_bytes()
        self.assertEqual(metrics["bottom_gap_px"], 0)
        self.assertEqual(before, after)
        self.assertFalse(metrics["modified"])

    def test_legacy_speckle_cleanup_removes_isolated_magenta(self):
        rgba = np.zeros((32, 32, 4), dtype=np.uint8)
        rgba[4:28, 6:26, :3] = (70, 120, 60)
        rgba[4:28, 6:26, 3] = 255
        rgba[16, 16, :3] = LEGACY_MAGENTA_RGB
        rgba[16, 16, 3] = 255
        cleaned, metrics = remove_isolated_key_speckles_rgba(rgba, key_rgb=LEGACY_MAGENTA_RGB)
        self.assertGreaterEqual(metrics["removed_pixels"], 1)
        self.assertEqual(int(cleaned[16, 16, 3]), 0)

    def test_ps1_stylizer_preserves_alpha_and_is_palette_locked(self):
        rgba = np.zeros((128, 128, 4), dtype=np.uint8)
        rgba[18:128, 28:104, :3] = (184, 105, 52)
        rgba[18:128, 28:104, 3] = 255
        palette = np.asarray([[72, 59, 42], [62, 62, 37], [61, 75, 38], [51, 78, 34]], dtype=np.float32)
        styled_a, metrics_a = stylize_card_rgba(rgba, palette, "trees")
        styled_b, metrics_b = stylize_card_rgba(rgba, palette, "trees")
        self.assertTrue(np.array_equal(styled_a, styled_b))
        self.assertEqual(metrics_a, metrics_b)
        self.assertTrue(np.array_equal(styled_a[..., 3], rgba[..., 3]))
        colors = np.unique(styled_a[rgba[..., 3] > 0, :3], axis=0)
        self.assertTrue(all(any(np.array_equal(color, item) for item in palette.astype(np.uint8)) for color in colors))


if __name__ == "__main__":
    unittest.main()
