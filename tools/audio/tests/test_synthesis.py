"""Tests de determinismo y humo para el pipeline de síntesis modular.

Mismo source + mismo código => mismo buffer (el repo exige determinismo).
"""

from __future__ import annotations

import numpy as np

from tools.audio.synthesis import (
    AdditiveSource,
    Analyzer,
    AntiAliasFilter,
    EffectsChain,
    EngineSynth,
    Envelope,
    LoudnessNormalizer,
    Resampler,
    WaveReader,
    WaveWriter,
)


def _build() -> EngineSynth:
    src = AdditiveSource(
        f0=120.0, duration_s=0.5, harmonic_amplitudes=[1.0, 0.5, 0.25, 0.12], seed=1
    )
    return EngineSynth(
        src,
        [
            Envelope(attack_s=0.01, decay_s=0.05, sustain_level=0.7, release_s=0.1),
            AntiAliasFilter(cutoff_hz=8000),
            LoudnessNormalizer(target_lufs=-14.0),
        ],
    )


def test_render_is_deterministic():
    a, _ = _build().render()
    b, _ = _build().render()
    assert a.shape == b.shape
    assert np.array_equal(a, b)


def test_resampler_changes_rate():
    sig, sr = EngineSynth(
        AdditiveSource(100.0, 0.2, [1.0, 0.3], seed=2), [Resampler(22050)]
    ).render()
    assert sr == 22050
    assert sig.shape[0] == round(0.2 * 22050)


def test_wave_roundtrip(tmp_path):
    out = tmp_path / "tone.wav"
    EngineSynth(AdditiveSource(80.0, 0.1, [1.0], seed=3), [WaveWriter(out)]).render_to_wav(out)
    assert out.is_file()
    sig, sr = EngineSynth(WaveReader(out)).render()
    assert sr == 44100
    assert sig.shape[0] > 0


def test_effects_chain_runs(tmp_path):
    out = tmp_path / "fx.wav"
    spec = tmp_path / "spec.png"
    synth = EngineSynth(
        AdditiveSource(110.0, 0.3, [1.0, 0.6, 0.4], seed=4),
        [
            Envelope(0.005, 0.05, 0.7, 0.08),
            AntiAliasFilter(7000.0),
            EffectsChain(reverb=0.3, compressor=True, pitch_semitones=-2.0),
            LoudnessNormalizer(-14.0),
            Analyzer(out_path=spec, kind="spectrogram"),
        ],
    )
    sig, sr = synth.render_to_wav(out)
    assert out.is_file()
    assert spec.is_file()
    assert np.all(np.isfinite(sig))
    assert sr == 44100
