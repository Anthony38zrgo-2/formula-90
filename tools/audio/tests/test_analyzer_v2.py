"""Self-contained V2 analyzer gates (numpy/scipy-free of audio I/O)."""

import numpy as np

from scripts.audio.analyze_engine_sweep import analyze_v2_signal

SR = 44100
RPM = 6000.0


def tone(order: float, seconds: float = 1.0) -> np.ndarray:
    t = np.arange(int(SR * seconds)) / SR
    return np.sin(2 * np.pi * RPM / 60 * order * t)


def test_v5_negative_half_block_dominates():
    report = analyze_v2_signal(tone(2.5), SR, RPM)
    assert report["half_block_dominance"]
    assert report["dominant_order"] == 2.5


def test_v10_positive_has_no_half_block_gate():
    report = analyze_v2_signal(tone(5.0), SR, RPM)
    assert not report["half_block_dominance"]
    assert report["dominant_order"] == 5.0


def test_dc_is_reported_separately_from_ac():
    report = analyze_v2_signal(0.6 + 0.1 * tone(5.0), SR, RPM)
    assert report["dc_offset"]
    assert report["ac_rms"] < report["rms"]


def test_sine_plus_noise_keeps_real_order_metrics():
    rng = np.random.default_rng(90)
    report = analyze_v2_signal(0.25 * tone(5.0) + 0.02 * rng.standard_normal(SR), SR, RPM)
    assert report["dominant_order"] == 5.0
    assert 5.0 in report["significant_orders"]
    assert report["sync_energy"] > 0.0
    assert report["residual_energy"] >= 0.0
    assert 0.0 <= report["sync_energy_ratio"] <= 1.0
    assert report["order_to_residual_db"] > 0.0
