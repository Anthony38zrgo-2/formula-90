"""Remaster de shift_up / shift_down — híbrido y no destructivo.

Conserva el cuerpo original (WaveReader) y le SUMA detalle: aire metálico
(MetalSheen), transitorio de engrane (ShiftTransient), y pega/brillo con
EffectsChain (compresor + distorsión + reverb). Finaliza con DC-removal y
clamp de pico para cumplir el contrato del banco (mono PCM16 44.1k, peak<0.99).

Todo determinista: mismos seeds => mismos bytes.
"""

from __future__ import annotations

import numpy as np

from tools.audio.synthesis.effects import EffectsChain
from tools.audio.synthesis.filters import HighPassFilter
from tools.audio.synthesis.io_audio import WaveReader, WaveWriter
from tools.audio.synthesis.loudness import LoudnessNormalizer
from tools.audio.synthesis.metal_sheen import MetalSheen
from tools.audio.synthesis.orchestrator import EngineSynth
from tools.audio.synthesis.protocols import Stage
from tools.audio.synthesis.shift_transient import ShiftTransient


class Gain(Stage):
    """Baja el cuerpo original para dejar headroom antes de sumar capas."""

    def __init__(self, gain: float = 0.7) -> None:
        self.gain = gain

    def process(self, x: np.ndarray, sr: int) -> tuple[np.ndarray, int]:
        return np.asarray(x, dtype=np.float64) * self.gain, sr


class Finalize(Stage):
    """Quita DC y clampa el pico (el contrato del banco exige peak<0.99)."""

    def __init__(self, peak: float = 0.92) -> None:
        self.peak = peak

    def process(self, x: np.ndarray, sr: int) -> tuple[np.ndarray, int]:
        y = np.asarray(x, dtype=np.float64)
        y = y - float(np.mean(y))
        mx = float(np.max(np.abs(y)))
        if mx > 1e-9:
            y = y * (self.peak / mx)
        return y, sr


def remaster_shift(src_wav, out_wav, kind: str = "up", target_lufs: float = -16.0) -> tuple[np.ndarray, int]:
    synth = EngineSynth(
        WaveReader(src_wav),
        [
            HighPassFilter(140.0, order=2),     # limpia rumble/DC
            Gain(0.7),                            # headroom
            MetalSheen(kind=kind, mix=0.18),     # aire metálico 4-9 kHz
            ShiftTransient(kind=kind, seed=7001),  # clack + blip (down)
            EffectsChain(compressor=True, distortion=4.0, reverb=0.12),  # glue + grit
            LoudnessNormalizer(target_lufs),
            Finalize(peak=0.92),
            WaveWriter(out_wav),
        ],
    )
    return synth.render_to_wav(out_wav)
