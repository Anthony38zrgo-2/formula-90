"""Stages de detalle para impactos (reutilizan el contrato Stage del paquete).

Cada stage añade una capa física concreta al cuerpo original:
* `ImpactBody`   -- thud low-mid (peso / golpe).
* `BoomSource`   -- sub con glissando descendente (explosión/fire).
* `CrackleLayer` -- pops tipo Poisson band-pass (whoosh / chispa / debris).
* `DebrisLayer`  -- ruido band-pass con tremolo lento (rozadura / grind).

Todos deterministas (seed).
"""

from __future__ import annotations

import numpy as np

from tools.audio.synthesis.filters import BandPassFilter
from tools.audio.synthesis.protocols import Stage
from tools.audio.synthesis.source import NoiseSource


class ImpactBody(Stage):
    """Thud low-mid con ligero pitch-drop y decay exponencial (da peso al golpe)."""

    def __init__(self, f0: float = 140.0, decay_s: float = 0.08, gain: float = 0.5, sub: bool = False, seed: int = 7401) -> None:
        self.f0 = f0
        self.decay_s = decay_s
        self.gain = gain
        self.sub = sub
        self.seed = seed

    def process(self, x: np.ndarray, sr: int) -> tuple[np.ndarray, int]:
        n = x.shape[0]
        t = np.arange(n) / sr
        denom = float(np.max(t)) if n > 1 else 1.0
        freq = self.f0 * (1.0 - 0.3 * (t / denom))  # pequeño drop de pitch
        body = np.sin(2.0 * np.pi * np.cumsum(freq) / sr) * np.exp(-t / self.decay_s)
        if self.sub:
            body = body + 0.6 * np.sin(2.0 * np.pi * (self.f0 * 0.4) * t) * np.exp(-t / (self.decay_s * 1.5))
        pk = float(np.max(np.abs(body)))
        if pk > 1e-9:
            body = body / pk
        return x + self.gain * body, sr


class BoomSource(Stage):
    """Sub con glissando descendente y decay largo (boom de explosión)."""

    def __init__(self, f0: float = 120.0, f1: float = 40.0, glide_s: float = 0.28, decay_s: float = 0.5, gain: float = 0.7, seed: int = 7301) -> None:
        self.f0 = f0
        self.f1 = f1
        self.glide_s = glide_s
        self.decay_s = decay_s
        self.gain = gain
        self.seed = seed

    def process(self, x: np.ndarray, sr: int) -> tuple[np.ndarray, int]:
        n = x.shape[0]
        t = np.arange(n) / sr
        tt = np.clip(t / self.glide_s, 0.0, 1.0)
        freq = self.f0 + (self.f1 - self.f0) * tt
        boom = np.sin(2.0 * np.pi * np.cumsum(freq) / sr) * np.exp(-t / self.decay_s)
        pk = float(np.max(np.abs(boom)))
        if pk > 1e-9:
            boom = boom / pk
        return x + self.gain * boom, sr


class CrackleLayer(Stage):
    """Pops tipo Poisson band-pass esparcidos en [t0, t1] (whoosh / chispa)."""

    def __init__(self, t0: float = 0.0, t1: float = 1.0, rate: float = 40.0, band: tuple[float, float] = (1000.0, 9000.0), gain: float = 0.3, seed: int = 7303) -> None:
        self.t0 = t0
        self.t1 = t1
        self.rate = rate
        self.band = band
        self.gain = gain
        self.seed = seed

    def process(self, x: np.ndarray, sr: int) -> tuple[np.ndarray, int]:
        n = x.shape[0]
        dur = n / sr
        rng = np.random.default_rng(self.seed)
        noise = NoiseSource(dur, color="white", seed=self.seed).render()
        noise = BandPassFilter(self.band[0], self.band[1]).process(noise, sr)[0]
        start = int(self.t0 * sr)
        end = int(min(self.t1, dur) * sr)
        layer = np.zeros(n, dtype=np.float64)
        if end > start:
            interval = max(1, int(sr / max(self.rate, 1e-3)))
            dk = max(1, int(0.003 * sr))
            pos = start
            while pos < end:
                jit = int(rng.integers(-interval // 2, interval // 2))
                p = pos + jit
                if 0 <= p < n:
                    for k in range(dk):
                        if p + k < n:
                            layer[p + k] = np.exp(-k / (0.001 * sr))
                pos += interval
        layer = layer * noise
        pk = float(np.max(np.abs(layer)))
        if pk > 1e-9:
            layer = layer / pk
        return x + self.gain * layer, sr


class DebrisLayer(Stage):
    """Ruido band-pass con tremolo lento sobre todo el buffer (rozadura / grind)."""

    def __init__(self, band: tuple[float, float] = (3000.0, 8000.0), depth: float = 0.5, gain: float = 0.12, seed: int = 7201) -> None:
        self.band = band
        self.depth = depth
        self.gain = gain
        self.seed = seed

    def process(self, x: np.ndarray, sr: int) -> tuple[np.ndarray, int]:
        n = x.shape[0]
        dur = n / sr
        noise = NoiseSource(dur, color="pink", seed=self.seed).render()
        noise = BandPassFilter(self.band[0], self.band[1]).process(noise, sr)[0]
        t = np.arange(n) / sr
        rng = np.random.default_rng(self.seed)
        phase = rng.uniform(0.0, 2.0 * np.pi)
        env = self.depth + (1.0 - self.depth) * np.abs(np.sin(2.0 * np.pi * 8.0 * t + phase))
        layer = noise * env
        pk = float(np.max(np.abs(layer)))
        if pk > 1e-9:
            layer = layer / pk
        return x + self.gain * layer, sr
