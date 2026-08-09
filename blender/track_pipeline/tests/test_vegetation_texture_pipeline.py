import sys
import unittest
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from analyze_vegetation_textures import visible_key_mask
from vegetation_texture_common import (
    DEFAULT_BACKGROUND_RGB,
    LEGACY_MAGENTA_RGB,
    prepare_vegetation_card_rgba,
    remove_isolated_key_speckles_rgba,
)


class VegetationPipelineValidationTests(unittest.TestCase):
    def test_validator_flags_visible_electric_cyan_key(self):
        rgba = np.zeros((8, 8, 4), dtype=np.uint8)
        rgba[2:6, 2:6, :3] = DEFAULT_BACKGROUND_RGB
        rgba[2:6, 2:6, 3] = 255
        self.assertEqual(int(visible_key_mask(rgba).sum()), 16)

    def test_validator_preserves_autumn_pink(self):
        rgba = np.zeros((8, 8, 4), dtype=np.uint8)
        rgba[2:6, 2:6, :3] = (220, 82, 168)
        rgba[2:6, 2:6, 3] = 255
        self.assertEqual(int(visible_key_mask(rgba).sum()), 0)

    def test_output_from_source_keyer_passes_visible_key_gate(self):
        img = np.empty((256, 256, 4), dtype=np.uint8)
        img[..., :3] = DEFAULT_BACKGROUND_RGB
        img[..., 3] = 255
        img[30:235, 55:200, :3] = (40, 118, 52)
        output, metrics = prepare_vegetation_card_rgba(img, (128, 128), DEFAULT_BACKGROUND_RGB, 1)
        self.assertEqual(metrics["bottom_gap_px"], 0)
        self.assertEqual(int(visible_key_mask(output).sum()), 0)

    def test_legacy_speckle_cleanup_removes_isolated_magenta(self):
        rgba = np.zeros((32, 32, 4), dtype=np.uint8)
        rgba[4:28, 6:26, :3] = (70, 120, 60)
        rgba[4:28, 6:26, 3] = 255
        rgba[16, 16, :3] = LEGACY_MAGENTA_RGB
        rgba[16, 16, 3] = 255
        cleaned, metrics = remove_isolated_key_speckles_rgba(rgba, key_rgb=LEGACY_MAGENTA_RGB)
        self.assertGreaterEqual(metrics["removed_pixels"], 1)
        self.assertEqual(int(cleaned[16, 16, 3]), 0)


if __name__ == "__main__":
    unittest.main()
