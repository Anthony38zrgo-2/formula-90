"""Remaster de impactos — perfiles por rol, sobre el toolkit modular.

Cada perfil es una cadena de stages (cuerpo original + capas de detalle).
Reutiliza `EngineSynth`, `ShiftTransient`, `MetalSheen`, los nuevos stages de
`impact_stages` y el `Gain`/`Finalize` de `remaster_shift`. Determinista.
"""

from __future__ import annotations

import numpy as np

from tools.audio.synthesis.effects import EffectsChain
from tools.audio.synthesis.filters import HighPassFilter
from tools.audio.synthesis.impact_stages import (
    BoomSource,
    CrackleLayer,
    DebrisLayer,
    ImpactBody,
)
from tools.audio.synthesis.io_audio import WaveReader, WaveWriter
from tools.audio.synthesis.loudness import LoudnessNormalizer
from tools.audio.synthesis.metal_sheen import MetalSheen
from tools.audio.synthesis.orchestrator import EngineSynth
from tools.audio.synthesis.remaster_shift import Finalize, Gain
from tools.audio.synthesis.shift_transient import ShiftTransient


def _stages(profile: str, target_lufs=None, **params) -> list:
    if profile == "barrier":
        tl = target_lufs if target_lufs is not None else -9.0
        return [
            HighPassFilter(120.0, order=2),
            Gain(0.7),
            ImpactBody(f0=160.0, decay_s=0.25, gain=0.35, sub=True, seed=7401),
            ShiftTransient(kind="up", base=2000.0, decay=0.06, clack_gain=0.5, seed=7101),
            DebrisLayer(band=(3000.0, 8000.0), depth=0.5, gain=0.12, seed=7201),
            EffectsChain(compressor=True, distortion=3.0, reverb=0.20),
            LoudnessNormalizer(tl),
            Finalize(0.92),
        ]
    if profile == "cone":
        tl = target_lufs if target_lufs is not None else -11.0
        return [
            HighPassFilter(150.0, order=2),
            Gain(0.7),
            ImpactBody(f0=320.0, decay_s=0.10, gain=0.5, seed=7402),
            ShiftTransient(kind="up", base=2600.0, decay=0.012, clack_gain=0.35, seed=7102),
            EffectsChain(compressor=True, distortion=2.0, reverb=0.10),
            LoudnessNormalizer(tl),
            Finalize(0.92),
        ]
    if profile == "fire":
        tl = target_lufs if target_lufs is not None else -10.0
        return [
            HighPassFilter(60.0, order=2),
            Gain(0.6),
            BoomSource(f0=120.0, f1=40.0, glide_s=0.28, decay_s=0.5, gain=0.7, seed=7301),
            ShiftTransient(kind="up", base=1800.0, decay=0.02, clack_gain=0.4, seed=7302),
            CrackleLayer(t0=0.0, t1=1.3, rate=40.0, band=(1000.0, 9000.0), gain=0.3, seed=7303),
            EffectsChain(compressor=True, distortion=4.0, reverb=0.25),
            LoudnessNormalizer(tl),
            Finalize(0.92),
        ]
    if profile == "hit":
        body_f0 = params.get("body_f0", 150.0)
        clang_base = params.get("clang_base", 2200.0)
        seed = params.get("seed", 7101)
        tl = target_lufs if target_lufs is not None else -12.0
        return [
            HighPassFilter(140.0, order=2),
            Gain(0.7),
            ImpactBody(f0=body_f0, decay_s=0.06, gain=0.5, sub=True, seed=seed + 100),
            ShiftTransient(kind="up", base=clang_base, decay=0.012, clack_gain=0.5, seed=seed),
            MetalSheen(mix=0.10, seed=seed + 200),
            EffectsChain(compressor=True, distortion=3.0, reverb=0.12),
            LoudnessNormalizer(tl),
            Finalize(0.92),
        ]
    if profile == "scrape":
        tl = target_lufs if target_lufs is not None else -16.0
        return [
            HighPassFilter(80.0, order=2),
            Gain(0.7),
            ImpactBody(f0=300.0, decay_s=0.12, gain=0.25, sub=False, seed=7403),
            MetalSheen(mix=0.06, seed=7202),
            EffectsChain(compressor=True, distortion=2.0, reverb=0.10),
            LoudnessNormalizer(tl),
            Finalize(0.92),
        ]
    raise ValueError(f"perfil de impacto desconocido: {profile}")


def remaster_impact(src_wav, out_wav, profile: str = "hit", target_lufs=None, **params) -> tuple[np.ndarray, int]:
    synth = EngineSynth(WaveReader(src_wav), _stages(profile, target_lufs, **params) + [WaveWriter(out_wav)])
    return synth.render_to_wav(out_wav)
