"""Stage de envolvente -- aplica un contorno de amplitud ADSR al buffer."""

from __future__ import annotations

import numpy as np

from tools.audio.synthesis.protocols import Stage


class Envelope(Stage):
    """Contorno ADSR (attack / decay / sustain / release) sobre toda la duración.

    Para un one-shot basta dejar ``total_s=None`` (usa la duración del buffer).
    Para un sonido de motor que hará loop, omitir este stage o usar una envolvente
    periódica aparte, para no meter caídas de amplitud en los bordes del loop.
    """

    def __init__(
        self,
        attack_s: float = 0.01,
        decay_s: float = 0.05,
        sustain_level: float = 0.8,
        release_s: float = 0.10,
        total_s: float | None = None,
    ) -> None:
        self.attack_s = attack_s
        self.decay_s = decay_s
        self.sustain_level = sustain_level
        self.release_s = release_s
        self.total_s = total_s

    def process(self, x: np.ndarray, sr: int) -> tuple[np.ndarray, int]:
        n = x.shape[0]
        env = np.ones(n, dtype=np.float64)
        na = int(self.attack_s * sr)
        nd = int(self.decay_s * sr)
        nr = int(self.release_s * sr)
        ns = max(n - na - nd - nr, 0)
        pos = 0
        if na > 0:
            env[pos : pos + na] = np.linspace(0.0, 1.0, na)
            pos += na
        if nd > 0:
            env[pos : pos + nd] = np.linspace(1.0, self.sustain_level, nd)
            pos += nd
        if ns > 0:
            env[pos : pos + ns] = self.sustain_level
            pos += ns
        if nr > 0 and pos < n:
            env[pos : pos + nr] = np.linspace(self.sustain_level, 0.0, max(nr, 1))[: n - pos]
        return x * env, sr
