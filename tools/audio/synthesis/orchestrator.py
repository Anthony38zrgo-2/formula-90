"""Orquestador -- compone un Source con una lista ordenada de Stages.

``EngineSynth`` es la única clase que conoce a los demás stages; los stages no
se conocen entre sí. Añade un stage con :meth:`EngineSynth.add` (fluent) o pásalos
en el constructor. Así se construyen sintezos complejos combinando fuentes y stages.

Ejemplo
-------
>>> from tools.audio.synthesis import (EngineSynth, AdditiveSource, Envelope,
...     AntiAliasFilter, EffectsChain, LoudnessNormalizer, Resampler, WaveWriter)
>>> synth = EngineSynth(
...     AdditiveSource(f0=120.0, duration_s=1.0,
...                    harmonic_amplitudes=[1.0, 0.6, 0.4, 0.25, 0.15]),
...     [Envelope(0.01, 0.05, 0.7, 0.1),
...      AntiAliasFilter(8000.0),
...      EffectsChain(reverb=0.3, compressor=True, pitch_semitones=-2.0),
...      LoudnessNormalizer(-14.0),
...      Resampler(44100)],
... )
>>> synth.render_to_wav("game/sounds/banks/v10_vehicle/engine_test.wav")
"""

from __future__ import annotations

from pathlib import Path

import numpy as np

from tools.audio.dsp_common import SAMPLE_RATE
from tools.audio.synthesis.analyze import Analyzer
from tools.audio.synthesis.effects import EffectsChain
from tools.audio.synthesis.envelope import Envelope
from tools.audio.synthesis.filters import (
    AntiAliasFilter,
    BandPassFilter,
    HighPassFilter,
    NotchFilter,
)
from tools.audio.synthesis.io_audio import WaveReader, WaveWriter
from tools.audio.synthesis.loudness import LoudnessNormalizer
from tools.audio.synthesis.protocols import Source, Stage
from tools.audio.synthesis.resample import Resampler
from tools.audio.synthesis.source import AdditiveSource, NoiseSource


class EngineSynth:
    """Encadena ``source`` + ``stages`` y produce el audio final."""

    def __init__(
        self,
        source: Source,
        stages: list[Stage] | None = None,
        sample_rate: int = SAMPLE_RATE,
    ) -> None:
        self.source = source
        self.stages: list[Stage] = list(stages or [])
        self.sample_rate = getattr(source, "sample_rate", sample_rate)

    def add(self, stage: Stage) -> EngineSynth:
        """Agrega un stage y devuelve self (encadenamiento fluido)."""
        self.stages.append(stage)
        return self

    def render(self) -> tuple[np.ndarray, int]:
        sig = self.source.render()
        sr = self.source.sample_rate
        for stage in self.stages:
            sig, sr = stage.process(sig, sr)
        return sig, sr

    def render_to_wav(self, path: str | Path, subtype: str = "PCM_16") -> tuple[np.ndarray, int]:
        sig, sr = self.render()
        WaveWriter(path, subtype).process(sig, sr)
        return sig, sr


__all__ = [
    "AdditiveSource",
    "Analyzer",
    "AntiAliasFilter",
    "BandPassFilter",
    "EffectsChain",
    "EngineSynth",
    "Envelope",
    "HighPassFilter",
    "LoudnessNormalizer",
    "NoiseSource",
    "NotchFilter",
    "Resampler",
    "Source",
    "Stage",
    "WaveReader",
    "WaveWriter",
]
