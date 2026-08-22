"""Stages de fuente: generan el buffer inicial.

* :class:`AdditiveSource` -- síntesis aditiva / Fourier (suma de parciales).
* :class:`NoiseSource`   -- ruido coloreado determinista (white / pink / brown).

Ambos son deterministas: cualquier aspecto estocástico (fase, ruido) recibe ``seed``.
"""

from __future__ import annotations

import numpy as np

from tools.audio.dsp_common import SAMPLE_RATE


class AdditiveSource:
    """Síntesis aditiva (Fourier) de un tono a partir de amplitudes armónicas."""

    def __init__(
        self,
        f0: float,
        duration_s: float,
        harmonic_amplitudes: np.ndarray | list[float],
        sample_rate: int = SAMPLE_RATE,
        inharmonicity: float = 0.0,
        phases: np.ndarray | None = None,
        seed: int | None = None,
    ) -> None:
        self.f0 = float(f0)
        self.duration_s = float(duration_s)
        self.sample_rate = int(sample_rate)
        self.amps = np.asarray(harmonic_amplitudes, dtype=np.float64)
        self.inharmonicity = float(inharmonicity)
        if phases is None:
            rng = np.random.default_rng(seed) if seed is not None else None
            self.phases = (
                rng.uniform(0.0, 2.0 * np.pi, self.amps.shape[0])
                if rng is not None
                else np.zeros(self.amps.shape[0])
            )
        else:
            self.phases = np.asarray(phases, dtype=np.float64)

    def render(self) -> np.ndarray:
        n = round(self.duration_s * self.sample_rate)
        t = np.arange(n, dtype=np.float64) / self.sample_rate
        out = np.zeros(n, dtype=np.float64)
        for k in range(self.amps.shape[0]):
            # Parcial inharmónico: f_k = f0*(k+1)*sqrt(1 + B*(k+1)^2)
            stretch = (k + 1) * np.sqrt(1.0 + self.inharmonicity * (k + 1) ** 2)
            freq = self.f0 * stretch
            out += self.amps[k] * np.sin(2.0 * np.pi * freq * t + self.phases[k])
        return out


class NoiseSource:
    """Fuente de ruido coloreado determinista (white / pink / brown)."""

    def __init__(
        self,
        duration_s: float,
        sample_rate: int = SAMPLE_RATE,
        color: str = "white",
        seed: int = 0,
    ) -> None:
        self.duration_s = float(duration_s)
        self.sample_rate = int(sample_rate)
        self.color = color
        self.seed = int(seed)

    def render(self) -> np.ndarray:
        n = round(self.duration_s * self.sample_rate)
        rng = np.random.default_rng(self.seed)
        white = rng.standard_normal(n)
        if self.color == "white":
            return white
        if self.color == "pink":
            # Aproximación barata de ruido rosa (filtro de Paul Kellet).
            pink = np.zeros(n, dtype=np.float64)
            b0 = b1 = b2 = b3 = b4 = b5 = b6 = 0.0
            for i in range(n):
                w = white[i]
                b0 = 0.99886 * b0 + w * 0.0555179
                b1 = 0.99332 * b1 + w * 0.0750759
                b2 = 0.96900 * b2 + w * 0.1538520
                b3 = 0.86650 * b3 + w * 0.3104856
                b4 = 0.55000 * b4 + w * 0.5329522
                b5 = -0.7616 * b5 - w * 0.0168980
                pink[i] = b0 + b1 + b2 + b3 + b4 + b5 + b6 + w * 0.5362
                b6 = w * 0.115926
            peak = float(np.max(np.abs(pink)))
            return pink / peak if peak > 1e-9 else pink
        if self.color == "brown":
            brown = np.cumsum(white)
            brown -= brown[0]
            peak = float(np.max(np.abs(brown)))
            return brown / peak if peak > 1e-9 else brown
        raise ValueError(f"unknown noise color: {self.color}")
