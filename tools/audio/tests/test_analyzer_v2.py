"""Self-contained V2 analyzer gates (numpy/scipy-free of audio I/O)."""

import numpy as np

from scripts.audio.analyze_engine_sweep import (
    _looks_like_pure_tone,
    _is_single_partial_dominant,
    analyze_v2_signal,
    segment_metrics,
    _band_energy_fraction,
    band_envelope_modulation,
    analyze_combustion_stems,
)

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


def test_pure_tone_gate_requires_concentration_and_sparse_orders():
    assert _looks_like_pure_tone(0.0001, 0.60, 0.99, 1)
    assert not _looks_like_pure_tone(0.0006, 0.23, 0.62, 15)


def test_single_partial_dominance_detects_oscillator_collapse():
    assert _is_single_partial_dominant(0.646)
    assert _is_single_partial_dominant(0.51)
    assert not _is_single_partial_dominant(0.2)
    assert not _is_single_partial_dominant(0.5)


def test_band_energy_fraction_ignores_out_of_band_tone():
    sr = 44100
    t = np.arange(int(sr)) / sr
    low = np.sin(2 * np.pi * 1000 * t)
    assert _band_energy_fraction(low, sr, 2500.0, 7000.0) < 0.05
    grit = np.sin(2 * np.pi * 4000 * t)
    assert _band_energy_fraction(grit, sr, 2500.0, 7000.0) > 0.9


def test_segment_reports_dual_mono_for_identical_channels():
    sr = 44100
    t = np.arange(int(sr)) / sr
    sig = np.sin(2 * np.pi * 1000 * t)
    stereo = np.stack([sig, sig], axis=1)
    m = segment_metrics(stereo, sr, 0.0, 1.0)
    assert m["max_lr_diff"] == 0.0
    assert m["dual_mono_identical"] is True or m["dual_mono_identical"]
    # A 1 kHz tone has no rasp-band content.
    assert m["energy_2500_7000"] < 0.05
    assert m["energy_700_2500"] > 0.9


def test_segment_flags_any_channel_difference():
    sr = 44100
    t = np.arange(int(sr)) / sr
    sig = np.sin(2 * np.pi * 1000 * t)
    right = sig.copy()
    right[500] += 1e-3  # a single differing sample
    stereo = np.stack([sig, right], axis=1)
    m = segment_metrics(stereo, sr, 0.0, 1.0)
    assert m["max_lr_diff"] > 0.0
    assert not m["dual_mono_identical"]


def test_segment_measures_rasp_band_on_mono():
    sr = 44100
    t = np.arange(int(sr)) / sr
    sig = np.sin(2 * np.pi * 4000 * t)
    stereo = np.stack([sig, sig], axis=1)
    m = segment_metrics(stereo, sr, 0.0, 1.0)
    assert m["energy_2500_7000"] > 0.9
    assert m["dual_mono_identical"]


def _write_stem(path, values):
    from scipy.io import wavfile

    wavfile.write(path, 44100, (np.clip(values, -1.0, 1.0) * 32767).astype(np.int16))


def test_combustion_stems_verify_residual_and_silence(tmp_path):
    sr = 44100
    rng = np.random.default_rng(7)
    pre_a = np.sin(2 * np.pi * 900 * np.arange(sr) / sr) * 0.4
    pre_b = rng.standard_normal(sr) * 0.2
    # Any split works for provenance as long as body + edge == pre_body.
    body_a = pre_a * 0.8
    edge_a = pre_a - body_a
    body_b = pre_b * 0.6
    edge_b = pre_b - body_b
    _write_stem(tmp_path / "pre_body_a.wav", pre_a)
    _write_stem(tmp_path / "pre_body_b.wav", pre_b)
    _write_stem(tmp_path / "combustion_body_a.wav", body_a)
    _write_stem(tmp_path / "combustion_body_b.wav", body_b)
    _write_stem(tmp_path / "combustion_edge_a.wav", edge_a)
    _write_stem(tmp_path / "combustion_edge_b.wav", edge_b)
    _write_stem(tmp_path / "combustion_body.wav", body_a + body_b)
    _write_stem(tmp_path / "combustion_edge.wav", edge_a + edge_b)
    for name in ("intake", "exhaust", "rasp"):
        _write_stem(tmp_path / f"{name}.wav", np.zeros(sr))

    report = analyze_combustion_stems(tmp_path)
    assert report["stems_present"]
    assert report["reconstruction_ok"], report["reconstruction_error"]
    assert report["edge_body_ratio"] > 0.0
    for name in ("intake", "exhaust", "rasp"):
        assert report[f"{name}_rms_exact_zero"] is True


def test_combustion_stems_reject_non_complementary_edge(tmp_path):
    # An edge from an oscillator/noise source does not rebuild pre_body, so the
    # provenance check must fail.
    sr = 44100
    t = np.arange(sr) / sr
    rng = np.random.default_rng(11)
    pre_a = np.sin(2 * np.pi * 900 * t) * 0.4
    body_a = pre_a * 0.8
    fake_edge_a = rng.standard_normal(sr) * 0.05
    _write_stem(tmp_path / "pre_body_a.wav", pre_a)
    _write_stem(tmp_path / "combustion_body_a.wav", body_a)
    _write_stem(tmp_path / "combustion_edge_a.wav", fake_edge_a)

    report = analyze_combustion_stems(tmp_path)
    assert not report["reconstruction_ok"]


def test_combustion_stems_flag_nonzero_disabled_voice(tmp_path):
    sr = 44100
    pre = np.sin(2 * np.pi * 900 * np.arange(sr) / sr) * 0.4
    body = pre * 0.8
    _write_stem(tmp_path / "pre_body_a.wav", pre)
    _write_stem(tmp_path / "combustion_body_a.wav", body)
    _write_stem(tmp_path / "combustion_edge_a.wav", pre - body)
    # A disabled voice that is merely quiet, not exactly silent, must be caught.
    # 1e-4 is above 1 LSB (3.05e-05), so it survives int16 quantisation.
    _write_stem(tmp_path / "rasp.wav", np.full(sr, 1e-4))

    report = analyze_combustion_stems(tmp_path)
    assert report["rasp_rms_exact_zero"] is False


def test_fixed_rpm_analysis_detects_injected_modulation():
    sr = 44100
    t = np.arange(4 * sr) / sr
    steady = np.sin(2 * np.pi * 4000 * t)
    clean = band_envelope_modulation(steady, sr)
    # Same carrier with a slow amplitude modulation injected.
    modulated = steady * (1.0 + 0.5 * np.sin(2 * np.pi * 6.64 * t))
    lfod = band_envelope_modulation(modulated, sr)

    assert lfod["envelope_cv"] > clean["envelope_cv"] * 3
    assert abs(lfod["mod_peak_hz"] - 6.64) < 0.5
    # A genuine LFO concentrates the modulation energy in one bin.
    assert lfod["mod_peak_to_median"] > clean["mod_peak_to_median"] * 5
