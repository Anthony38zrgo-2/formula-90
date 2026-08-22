"""Stage de re-muestreo -- cambia el sample rate con resampy (alta calidad)."""

from __future__ import annotations

import numpy as np
import resampy

from tools.audio.synthesis.protocols import Stage


class Resampler(Stage):
    def __init__(self, target_sr: int) -> None:
        self.target_sr = int(target_sr)

    def process(self, x: np.ndarray, sr: int) -> tuple[np.ndarray, int]:
        if sr == self.target_sr:
            return x, sr
        y = resampy.resample(x, sr, self.target_sr)
        return y, self.target_sr
