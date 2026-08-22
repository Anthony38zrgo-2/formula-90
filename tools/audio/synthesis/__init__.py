"""Síntesis de audio modular y headless para Formula-90.

Compón un :class:`Source` (p.ej. :class:`AdditiveSource`) con una lista ordenada
de objetos :class:`Stage`, y deja que :class:`EngineSynth` los ejecute. Cada stage
vive en su propio módulo, así que añadir DSP nuevo no requiere tocar el orquestador.
"""

from __future__ import annotations

from tools.audio.synthesis.analyze import Analyzer
from tools.audio.synthesis.effects import EffectsChain
from tools.audio.synthesis.envelope import Envelope
from tools.audio.synthesis.filters import (
    AntiAliasFilter,
    BandPassFilter,
    HighPassFilter,
    NotchFilter,
)
from tools.audio.synthesis.impact_stages import BoomSource, CrackleLayer, DebrisLayer, ImpactBody
from tools.audio.synthesis.io_audio import WaveReader, WaveWriter
from tools.audio.synthesis.loudness import LoudnessNormalizer
from tools.audio.synthesis.metal_sheen import MetalSheen
from tools.audio.synthesis.orchestrator import EngineSynth
from tools.audio.synthesis.protocols import Source, Stage
from tools.audio.synthesis.remaster_impact import remaster_impact
from tools.audio.synthesis.remaster_shift import Finalize, Gain, remaster_shift
from tools.audio.synthesis.resample import Resampler
from tools.audio.synthesis.shift_transient import ShiftTransient
from tools.audio.synthesis.source import AdditiveSource, NoiseSource

__all__ = [
    "AdditiveSource",
    "Analyzer",
    "AntiAliasFilter",
    "BandPassFilter",
    "BoomSource",
    "CrackleLayer",
    "DebrisLayer",
    "EffectsChain",
    "EngineSynth",
    "Envelope",
    "Finalize",
    "Gain",
    "HighPassFilter",
    "ImpactBody",
    "LoudnessNormalizer",
    "MetalSheen",
    "NoiseSource",
    "NotchFilter",
    "Resampler",
    "ShiftTransient",
    "Source",
    "Stage",
    "WaveReader",
    "WaveWriter",
    "remaster_impact",
    "remaster_shift",
]
