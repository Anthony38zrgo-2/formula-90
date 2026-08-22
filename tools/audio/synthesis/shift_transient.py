"""Stage de transitorio de cambio de marcha.

Añade el "clack" metálico de engrane (partiales inharmónicos con decay rápido)
en t=0, y para *downshift* un "blip" ascendente de gas ~40 ms después, que es
el sonido característico de la punta de embrague al reducir. Determinista (seed).
"""

from __future__ import annotations

import numpy as np

from tools.audio.synthesis.filters import BandPassFilter
from tools.audio.synthesis.protocols import Stage


class ShiftTransient(Stage):
    def __init__(
        self,
        kind: str = "up",
        seed: int = 7001,
        clack_gain: float = 0.5,
        blip_gain: float = 0.35,
        base: float = 2600.0,
        decay: float = 0.010,
    ) -> None:
        self.kind = kind
        self.seed = seed
        self.clack_gain = clack_gain
        self.blip_gain = blip_gain
        self.base = base
        self.decay = decay

    def process(self, x: np.ndarray, sr: int) -> tuple[np.ndarray, int]:
        n = x.shape[0]
        out = np.array(x, dtype=np.float64)
        t = np.arange(n) / sr
        rng = np.random.default_rng(self.seed)

        # Clack metálico: partiales inharmónicos con decay exponencial (ajustable).
        base = self.base
        ratios = np.array([1.0, 1.48, 2.13, 2.74, 3.51, 4.32])
        amps = np.array([1.0, 0.7, 0.5, 0.35, 0.22, 0.14])
        phases = rng.uniform(0.0, 2.0 * np.pi, ratios.shape[0])
        decay = self.decay
        clack = np.zeros(n, dtype=np.float64)
        for r, a, ph in zip(ratios, amps, phases):
            f = base * r
            clack += a * np.sin(2.0 * np.pi * f * t + ph) * np.exp(-t / decay)
        clack = BandPassFilter(1800, 6500).process(clack, sr)[0]
        pk = float(np.max(np.abs(clack)))
        if pk > 1e-9:
            clack = clack / pk
        out += self.clack_gain * clack

        # Downshift: blip ascendente (rev) ~40 ms después del clack.
        if self.kind == "down":
            start = int(0.04 * sr)
            blen = int(0.035 * sr)
            if start + blen <= n:
                tt = np.arange(blen) / sr
                f0, f1 = 1200.0, 2600.0
                freq = f0 + (f1 - f0) * (tt / blen)
                blip = np.sin(2.0 * np.pi * np.cumsum(freq) / sr)
                env = np.exp(-tt / 0.012)
                blip = blip * env
                bpk = float(np.max(np.abs(blip)))
                if bpk > 1e-9:
                    blip = blip / bpk
                out[start : start + blen] += self.blip_gain * blip

        return out, sr
