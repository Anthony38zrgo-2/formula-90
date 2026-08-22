"""Stage de "aire" metálico para shift (lo que les falta: detalle > 4 kHz).

Ruido rosa -> band-pass 4-9 kHz con envolvente de ataque rápido y cola suave,
sumado al cuerpo original. Aporta el brillo/cresta metálica que el sample
original no tiene (energía > 8 kHz era 0.0). Determinista (seed).
"""

from __future__ import annotations

import numpy as np

from tools.audio.synthesis.filters import BandPassFilter
from tools.audio.synthesis.protocols import Stage
from tools.audio.synthesis.source import NoiseSource


class MetalSheen(Stage):
    def __init__(self, kind: str = "up", mix: float = 0.18, seed: int = 7101) -> None:
        self.kind = kind
        self.mix = mix
        self.seed = seed

    def process(self, x: np.ndarray, sr: int) -> tuple[np.ndarray, int]:
        n = x.shape[0]
        dur = n / sr
        noise = NoiseSource(dur, color="pink", seed=self.seed).render()
        noise = BandPassFilter(4000, 9000).process(noise, sr)[0]

        # Envolvente: ataque 4 ms (arranca en 0.2 para no clickear), deca 50 ms.
        env = np.full(n, 0.25, dtype=np.float64)
        na = int(0.004 * sr)
        nd = int(0.05 * sr)
        if na > 0:
            env[:na] = np.linspace(0.2, 1.0, na)
        if nd > 0:
            env[na : na + nd] = np.linspace(1.0, 0.25, nd)

        noise = noise * env
        pk = float(np.max(np.abs(noise)))
        if pk > 1e-9:
            noise = noise / pk
        return x + self.mix * noise, sr
