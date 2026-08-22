"""Stage de loudness -- normalización EBU R128 con pyloudnorm (cohesión de banco)."""

from __future__ import annotations

import numpy as np

from tools.audio.synthesis.protocols import Stage

try:
    import pyloudnorm as pyln

    _HAS_PYLN = True
except Exception:  # noqa: BLE001 - pragma: no cover - degrada a identidad si no está instalado
    pyln = None
    _HAS_PYLN = False


class LoudnessNormalizer(Stage):
    def __init__(self, target_lufs: float = -14.0) -> None:
        self.target_lufs = target_lufs

    def process(self, x: np.ndarray, sr: int) -> tuple[np.ndarray, int]:
        if not _HAS_PYLN:
            return x, sr
        try:
            meter = pyln.Meter(sr)  # type: ignore[attr-defined]
            loud = meter.integrated_loudness(x)
            if not np.isfinite(loud):
                return x, sr
            y = pyln.normalize.loudness(x, loud, self.target_lufs)  # type: ignore[attr-defined]
            return np.clip(y, -1.0, 1.0), sr
        except Exception:  # noqa: BLE001
            return x, sr
