#!/usr/bin/env python3
"""Measure an offline procedural-engine sweep and produce JSON/PNG evidence."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import subprocess
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np
from scipy import signal
from scipy.io import wavfile

EPS = 1e-15


def _looks_like_pure_tone(flatness: float, top1: float, top10: float,
                          significant_order_count: int) -> bool:
    """Distinguish an oscillator from a harmonic, impulse-driven engine tone."""
    return (
        flatness < 0.008
        and top1 > 0.18
        and top10 > 0.92
        and significant_order_count <= 2
    )


def _is_single_partial_dominant(fundamental_ratio: float, threshold: float = 0.5) -> bool:
    """The delayed-impulse engine must spread energy over many harmonics. When
    the fundamental partial alone carries more than `threshold` of the band
    energy, the impulse train has collapsed into a sinusoidal oscillator."""
    return fundamental_ratio > threshold


def _order_energy(mono: np.ndarray, sr: int, rpm: float, order: float) -> float:
    """Energy in a narrow, detrended FFT band around an engine order."""
    if mono.size < 32 or rpm <= 0:
        return 0.0
    x = mono - np.mean(mono)
    window = np.hanning(x.size)
    spectrum = np.fft.rfft(x * window)
    frequencies = np.fft.rfftfreq(x.size, 1.0 / sr)
    target = rpm / 60.0 * order
    width = max(2.0, target * 0.035)
    selected = np.abs(frequencies - target) <= width
    return float(np.sum(np.abs(spectrum[selected]) ** 2)) if np.any(selected) else 0.0


def analyze_v2_metrics(mono: np.ndarray, sr: int, rpm: float) -> dict[str, float | bool]:
    """Return DC/AC and V10 order gates for one fixed-RPM or analysis window."""
    mean = float(np.mean(mono)) if mono.size else 0.0
    rms = float(np.sqrt(np.mean(np.square(mono)))) if mono.size else 0.0
    ac = mono - mean
    ac_rms = float(np.sqrt(np.mean(np.square(ac)))) if mono.size else 0.0
    e25 = _order_energy(mono, sr, rpm, 2.5)
    e5 = _order_energy(mono, sr, rpm, 5.0)
    ratio_db = 10.0 * math.log10(max(e25, EPS) / max(e5, EPS))
    # Half-block leakage is the excess 2.5-order energy over the expected V10 order.
    return {"mean": mean, "rms": rms, "ac_rms": ac_rms,
            "dc_ratio": abs(mean) / max(rms, EPS),
            "dc_to_ac_db": 20.0 * math.log10(max(abs(mean), EPS) / max(ac_rms, EPS)),
            "engine_order_2_5_energy": e25, "engine_order_5_energy": e5,
            "half_block_leakage_db": ratio_db,
            "half_block_dominance": ratio_db > 3.0,
            "dc_offset": abs(mean) / max(rms, EPS) > 0.05}


DEFAULT_ENGINE_ORDERS = (0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0, 7.5, 10.0, 12.5, 15.0, 20.0, 25.0)


def analyze_v2_signal(signal_in: np.ndarray, sr: int, rpm: float,
                      orders: tuple[float, ...] = DEFAULT_ENGINE_ORDERS) -> dict:
    """Analyze one near-stationary window and return real order metrics."""
    base = analyze_v2_metrics(signal_in, sr, rpm)
    x = signal_in - np.mean(signal_in)
    spectrum = np.abs(np.fft.rfft(x * np.hanning(x.size))) ** 2
    freqs = np.fft.rfftfreq(x.size, 1.0 / sr)
    energies = {float(o): _order_energy(signal_in, sr, rpm, o) for o in orders}
    # Union of order bands: overlapping skirts are counted once only.
    order_freqs = np.asarray([rpm / 60.0 * o for o in orders])
    band_widths = np.maximum(2.0, order_freqs * 0.035)
    sync_mask = np.zeros(freqs.shape, dtype=bool)
    for centre, width in zip(order_freqs, band_widths):
        sync_mask |= np.abs(freqs - centre) <= width
    total_band_mask = (freqs >= max(1.0, order_freqs.min() - band_widths.max())) & (freqs <= order_freqs.max() + band_widths.max())
    sync_energy = float(np.sum(spectrum[sync_mask]))
    total_band_energy = float(np.sum(spectrum[total_band_mask]))
    residual_energy = max(0.0, total_band_energy - sync_energy)
    peak = max(energies.values(), default=0.0)
    significant = [o for o, energy in energies.items() if peak and energy >= peak * 10 ** (-24.0 / 10.0)]
    dominant = max(energies, key=energies.get) if peak else None
    base.update({"significant_orders": significant, "dominant_order": dominant,
                 "order_energies": energies,
                 "sync_energy": sync_energy, "residual_energy": residual_energy,
                 "sync_energy_ratio": sync_energy / max(total_band_energy, EPS),
                 "residual_energy_ratio": residual_energy / max(total_band_energy, EPS),
                 "order_to_residual_db": 10.0 * math.log10(max(sync_energy, EPS) / max(residual_energy, EPS))})
    return base


def dbfs(value: float) -> float:
    return 20.0 * math.log10(max(abs(value), EPS))


def band_envelope_modulation(
    mono: np.ndarray,
    sr: int,
    band_lo: float = 2500.0,
    band_hi: float = 7000.0,
    mod_lo: float = 0.2,
    mod_hi: float = 30.0,
) -> dict[str, float]:
    """Modulation of the 2.5-7 kHz envelope, for fixed-RPM renders.

    A real LFO/phaser shows up as a *dominant* narrow peak at the same
    frequency regardless of RPM. Natural event jitter spreads the modulation
    energy and the peak frequency moves with RPM, so this reports both the
    coefficient of variation and how dominant the strongest peak is.
    """
    band = signal.sosfiltfilt(
        signal.butter(4, [band_lo / (sr / 2), band_hi / (sr / 2)], btype="band", output="sos"),
        mono,
    )
    envelope = np.abs(signal.hilbert(band))
    envelope = signal.sosfiltfilt(
        signal.butter(4, 60.0 / (sr / 2), btype="low", output="sos"), envelope
    )
    mean = float(np.mean(envelope))
    cv = float(np.std(envelope) / max(mean, EPS))
    detrended = envelope - mean
    spectrum = np.abs(np.fft.rfft(detrended * np.hanning(detrended.size))) ** 2
    freqs = np.fft.rfftfreq(detrended.size, 1.0 / sr)
    selected = (freqs >= mod_lo) & (freqs <= mod_hi)
    if not np.any(selected):
        return {"envelope_cv": cv, "mod_peak_hz": 0.0, "mod_peak_ratio": 0.0,
                "mod_peak_to_median": 0.0}
    power = spectrum[selected]
    peak = int(np.argmax(power))
    median = float(np.median(power))
    return {
        "envelope_cv": cv,
        "mod_peak_hz": float(freqs[selected][peak]),
        "mod_peak_ratio": float(power[peak] / max(power.sum(), EPS)),
        "mod_peak_to_median": float(power[peak] / max(median, EPS)),
    }


def analyze_combustion_stems(stem_dir: Path) -> dict:
    """Provenance checks for the two declared combustion voices."""
    def read(name: str) -> np.ndarray | None:
        path = stem_dir / f"{name}.wav"
        if not path.exists():
            return None
        _, raw = wavfile.read(path)
        return raw.astype(np.float64) / 32767.0

    result: dict = {"stems_present": False}

    def rms(x: np.ndarray) -> float:
        return dbfs(float(np.sqrt(np.mean(np.square(x)))))

    body = read("combustion_body")
    edge = read("combustion_edge")
    if body is not None and edge is not None:
        result["stems_present"] = True
        result["combustion_body_rms_dbfs"] = rms(body)
        result["combustion_edge_rms_dbfs"] = rms(edge)
        result["edge_body_ratio"] = float(
            np.sqrt(np.mean(np.square(edge))) / max(np.sqrt(np.mean(np.square(body))), EPS)
        )

    # Residual complementarity: body + edge must rebuild pre_body per sample.
    # Checked per bank, independently of whether the summed stems exist.
    worst = 0.0
    checked = False
    for bank in ("a", "b"):
        pre = read(f"pre_body_{bank}")
        body_bank = read(f"combustion_body_{bank}")
        edge_bank = read(f"combustion_edge_{bank}")
        if pre is None or body_bank is None or edge_bank is None:
            continue
        checked = True
        worst = max(worst, float(np.max(np.abs(pre - (body_bank + edge_bank)))))
    # 2 LSB of int16 is the expected f32/PCM tolerance.
    result["reconstruction_error"] = worst
    result["reconstruction_ok"] = checked and worst <= 2.0 / 32767.0

    # Disabled voices must be exact silence, not merely quiet.
    for name in ("intake", "exhaust", "rasp"):
        values = read(name)
        result[f"{name}_rms_exact_zero"] = (
            bool(np.all(values == 0.0)) if values is not None else None
        )
        if values is not None:
            result["stems_present"] = True
    return result


def load_telemetry(path: Path) -> dict[str, np.ndarray]:
    with path.open("r", newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    return {
        key: np.asarray([float(row[key]) for row in rows], dtype=np.float64)
        for key in ("time_s", "rpm", "throttle", "rms_l", "rms_r", "peak")
    }


def _band_energy_fraction(signal_mono: np.ndarray, sr: int, lo: float, hi: float) -> float:
    """Fraction of total energy contained in the [lo, hi] Hz band (mono)."""
    x = signal_mono - np.mean(signal_mono)
    spectrum = np.abs(np.fft.rfft(x * np.hanning(x.size))) ** 2
    freqs = np.fft.rfftfreq(x.size, 1.0 / sr)
    band = (freqs >= lo) & (freqs <= hi)
    return float(np.sum(spectrum[band]) / max(np.sum(spectrum), EPS))


def segment_metrics(stereo: np.ndarray, sr: int, start: float, end: float) -> dict[str, float]:
    """Per-segment metrics for a dual-mono engine.

    Everything musical is measured on the real mono signal. Channel agreement is
    reported as an exact sample-domain difference, not as a correlation: the
    engine is required to be bit-identical, so any non-zero difference is a bug.
    """
    part = stereo[int(start * sr) : int(end * sr)]
    mono = part.mean(axis=1)
    rms = float(np.sqrt(np.mean(np.square(mono), dtype=np.float64)))
    peak = float(np.max(np.abs(part)))
    crest = dbfs(peak / max(rms, EPS))
    # Exact L/R disagreement in the sample domain (0.0 == dual-mono).
    max_lr_diff = float(np.max(np.abs(part[:, 0] - part[:, 1])))
    return {
        "start_s": start,
        "end_s": end,
        "rms_dbfs": dbfs(rms),
        "peak_dbfs": dbfs(peak),
        "crest_db": crest,
        "max_lr_diff": max_lr_diff,
        "dual_mono_identical": max_lr_diff == 0.0,
        # Approved mid layer 700 Hz - 2.5 kHz.
        "energy_700_2500": _band_energy_fraction(mono, sr, 700.0, 2500.0),
        # Structured grit (rasp) band 2.5-7 kHz, measured on mono.
        "energy_2500_7000": _band_energy_fraction(mono, sr, 2500.0, 7000.0),
    }


def analyze(
    wav_path: Path, telemetry_path: Path, stem_dir: Path | None = None
) -> tuple[dict, dict[str, np.ndarray]]:
    sr, raw = wavfile.read(wav_path)
    if raw.ndim != 2 or raw.shape[1] != 2:
        raise ValueError("expected stereo WAV")
    scale = float(np.iinfo(raw.dtype).max) if np.issubdtype(raw.dtype, np.integer) else 1.0
    stereo = raw.astype(np.float64) / scale
    mono = stereo.mean(axis=1)
    telemetry = load_telemetry(telemetry_path)

    # V2 uses short, detrended windows so a sweep is compared at a nearly fixed
    # RPM. This prevents the changing pitch from hiding half-block leakage.
    v2_windows: list[dict] = []
    window_size = max(1024, int(sr * 0.5))
    for start in range(0, mono.size - window_size + 1, max(1, window_size // 2)):
        centre_t = (start + window_size / 2) / sr
        rpm_c = float(np.interp(centre_t, telemetry["time_s"], telemetry["rpm"]))
        v2_windows.append(analyze_v2_signal(mono[start:start + window_size], sr, rpm_c))
    if v2_windows:
        scalar_keys = ("mean", "rms", "ac_rms", "dc_ratio", "dc_to_ac_db", "engine_order_2_5_energy",
                       "engine_order_5_energy", "half_block_leakage_db", "sync_energy", "residual_energy",
                       "sync_energy_ratio", "residual_energy_ratio", "order_to_residual_db")
        v2 = {key: float(np.nanmedian([float(w[key]) for w in v2_windows])) for key in scalar_keys}
        v2["half_block_dominance"] = bool(np.any([w["half_block_dominance"] for w in v2_windows]))
        v2["dc_offset"] = bool(np.any([w["dc_offset"] for w in v2_windows]))
        all_orders = [o for w in v2_windows for o in w["significant_orders"]]
        v2["significant_orders"] = sorted(set(all_orders))
        v2["dominant_order"] = float(np.median([w["dominant_order"] for w in v2_windows if w["dominant_order"] is not None]))
    else:
        v2 = analyze_v2_signal(mono, sr, float(np.median(telemetry["rpm"])))
    channel_v2 = {name: analyze_v2_metrics(stereo[:, idx], sr, float(np.median(telemetry["rpm"])))
                  for idx, name in enumerate(("left", "right"))}

    frequencies, times, spectrum = signal.stft(
        mono,
        fs=sr,
        window="hann",
        nperseg=4096,
        noverlap=3072,
        boundary=None,
        padded=False,
    )
    power = np.square(np.abs(spectrum))
    band = (frequencies >= 40.0) & (frequencies <= 10_000.0)
    band_power = power[band] + EPS
    band_freq = frequencies[band]
    total_power = np.sum(band_power, axis=0)
    flatness = np.exp(np.mean(np.log(band_power), axis=0)) / np.mean(band_power, axis=0)
    top1 = np.max(band_power, axis=0) / total_power
    sorted_power = np.sort(band_power, axis=0)
    top10 = np.sum(sorted_power[-10:], axis=0) / total_power
    centroid = np.sum(band_freq[:, None] * band_power, axis=0) / total_power

    rpm = np.interp(times, telemetry["time_s"], telemetry["rpm"])
    expected_firing = rpm / 12.0
    observed_firing = np.empty_like(times)
    firing_error = np.empty_like(times)
    fundamental_ratio = np.empty_like(times)
    for index, expected in enumerate(expected_firing):
        width = max(22.0, expected * 0.12)
        local = (band_freq >= expected - width) & (band_freq <= expected + width)
        if not np.any(local):
            observed_firing[index] = np.nan
            firing_error[index] = np.nan
            fundamental_ratio[index] = 0.0
            continue
        local_power = band_power[local, index]
        local_freq = band_freq[local]
        strongest = int(np.argmax(local_power))
        observed_firing[index] = local_freq[strongest]
        firing_error[index] = abs(observed_firing[index] - expected) / expected * 100.0
        fundamental_ratio[index] = float(np.sum(local_power) / total_power[index])

    # Measure time-domain RMS around the STFT centres. STFT magnitudes use window
    # normalization and are useful for spectral ratios, but are not an absolute
    # dBFS meter.
    half_window = 2048
    frame_rms = np.empty_like(times)
    for index, time_s in enumerate(times):
        centre = round(time_s * sr)
        lo = max(0, centre - half_window)
        hi = min(mono.size, centre + half_window)
        frame_rms[index] = np.sqrt(np.mean(np.square(mono[lo:hi]), dtype=np.float64))
    frame_rms_dbfs = 20.0 * np.log10(np.maximum(frame_rms, EPS))
    segments = {
        "idle": segment_metrics(stereo, sr, 0.5, 1.5),
        "low": segment_metrics(stereo, sr, 2.0, 4.0),
        "mid": segment_metrics(stereo, sr, 4.0, 6.0),
        "high": segment_metrics(stereo, sr, 6.0, 8.0),
        "redline": segment_metrics(stereo, sr, 8.5, 10.0),
    }
    for metrics in segments.values():
        selected = (times >= metrics["start_s"]) & (times < metrics["end_s"])
        metrics["median_spectral_flatness"] = float(np.median(flatness[selected]))
        metrics["median_top_bin_energy_ratio"] = float(np.median(top1[selected]))
        metrics["median_top_10_bins_energy_ratio"] = float(np.median(top10[selected]))
        metrics["median_spectral_centroid_hz"] = float(np.median(centroid[selected]))
        metrics["median_firing_error_percent"] = float(np.nanmedian(firing_error[selected]))
    clipping_fraction = float(np.mean(np.abs(stereo) >= 0.999))
    valid = np.isfinite(firing_error)
    diagnostics: list[dict[str, str]] = []

    def flag(code: str, severity: str, message: str) -> None:
        diagnostics.append({"code": code, "severity": severity, "message": message})

    idle_level = segments["idle"]["rms_dbfs"]
    if idle_level < -70.0:
        flag("idle_silent", "critical", f"Idle RMS is effectively silent ({idle_level:.1f} dBFS).")
    elif idle_level < -45.0:
        flag("idle_inaudible", "high", f"Idle RMS is below the perceptual gate ({idle_level:.1f} dBFS).")
    if segments["low"]["rms_dbfs"] < -35.0:
        flag("low_rpm_quiet", "medium", f"Low-RPM RMS remains weak ({segments['low']['rms_dbfs']:.1f} dBFS).")
    dynamic_range = segments["high"]["rms_dbfs"] - idle_level
    if dynamic_range > 35.0:
        flag("excessive_level_curve", "high", f"Idle-to-high level span is {dynamic_range:.1f} dB; the load curve is too steep.")
    median_flatness = float(np.median(flatness))
    median_top1 = float(np.median(top1))
    median_top10 = float(np.median(top10))
    if median_flatness > 0.35:
        flag("noise_dominant", "high", f"Median spectral flatness {median_flatness:.3f} indicates noise dominance.")
    high_flatness = max(
        segments["high"]["median_spectral_flatness"],
        segments["redline"]["median_spectral_flatness"],
    )
    if high_flatness > 0.30:
        flag(
            "high_rpm_noise",
            "high",
            f"High-RPM flatness reaches {high_flatness:.3f}; turbulence is approaching noise dominance.",
        )
    if _looks_like_pure_tone(
        median_flatness,
        median_top1,
        median_top10,
        len(v2["significant_orders"]),
    ):
        flag("pure_tone", "high", "Energy is concentrated like a near-pure oscillator.")
    if median_top10 > 0.92:
        flag("spectral_concentration", "medium", f"Ten FFT bins contain {median_top10 * 100.0:.1f}% of energy.")
    median_fundamental_ratio = float(np.median(fundamental_ratio))
    if _is_single_partial_dominant(median_fundamental_ratio):
        flag(
            "single_partial_dominance",
            "high",
            f"The firing-frequency partial contains {median_fundamental_ratio * 100.0:.1f}% of "
            "the band energy: the impulse train degenerated into a sinusoidal oscillator.",
        )
    median_tracking = float(np.nanmedian(firing_error[valid])) if np.any(valid) else float("nan")
    if not math.isfinite(median_tracking) or median_tracking > 8.0:
        flag("firing_mismatch", "high", f"Median V10 firing-frequency error is {median_tracking:.1f}%.")
    # Mono is now a requirement for the continuous engine, not a defect, so
    # `stereo_collapse` is intentionally gone. What IS a defect is any L/R
    # difference at all: the engine must be bit-identical on both channels.
    for seg_name in ("idle", "low", "mid", "high", "redline"):
        diff = float(segments[seg_name]["max_lr_diff"])
        if diff != 0.0:
            flag(
                "dual_mono_violation",
                "critical",
                f"{seg_name} channels differ by up to {diff:g}; the procedural engine "
                f"must be dual-mono (L == R bit for bit).",
            )
    if segments["high"]["crest_db"] < 3.5:
        flag("high_rpm_flat_envelope", "medium", f"High-RPM crest factor is only {segments['high']['crest_db']:.1f} dB.")
    if clipping_fraction > 0.0001:
        flag("clipping", "high", f"Clipping affects {clipping_fraction * 100.0:.3f}% of samples.")
    if bool(v2["dc_offset"]):
        flag("dc_offset", "critical", f"V2 DC ratio is {float(v2['dc_ratio']):.3f}; signal is not centred.")
    if bool(v2["half_block_dominance"]):
        flag("half_block_dominance", "critical", f"2.5-order exceeds 5th order by {float(v2['half_block_leakage_db']):.1f} dB.")
    if not diagnostics:
        flag("no_heuristic_failure", "info", "No configured numerical gate failed; human timbre review is still required.")

    try:
        git_sha = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    except (OSError, subprocess.CalledProcessError):
        git_sha = "unknown"
    metadata = {"git_sha": git_sha, "wav_sha256": hashlib.sha256(wav_path.read_bytes()).hexdigest(),
                "telemetry_sha256": hashlib.sha256(telemetry_path.read_bytes()).hexdigest(),
                "analyzer": "v2", "config_sha256": hashlib.sha256(json.dumps({"sample_rate": int(sr), "window_s": 0.5}, sort_keys=True).encode()).hexdigest()}
    result = {
        "source_wav": str(wav_path.resolve()),
        "sample_rate_hz": int(sr),
        "frames": int(stereo.shape[0]),
        "duration_s": float(stereo.shape[0] / sr),
        "expected_model": "V10 four-stroke; acoustic firing frequency = RPM / 12",
        "global": {
            "rms_dbfs": dbfs(float(np.sqrt(np.mean(np.square(mono))))),
            "peak_dbfs": dbfs(float(np.max(np.abs(stereo)))),
            "clipping_fraction": clipping_fraction,
            "median_spectral_flatness": median_flatness,
            "median_top_bin_energy_ratio": median_top1,
            "median_top_10_bins_energy_ratio": median_top10,
            "median_spectral_centroid_hz": float(np.median(centroid)),
            "median_firing_frequency_error_percent": median_tracking,
            "median_fundamental_band_energy_ratio": median_fundamental_ratio,
            "single_partial_dominance": bool(_is_single_partial_dominant(median_fundamental_ratio)),
            "idle_to_high_level_span_db": dynamic_range,
            "rms_mono_dbfs": dbfs(float(np.sqrt(np.mean(np.square(mono))))),
            "max_lr_diff": float(np.max(np.abs(stereo[:, 0] - stereo[:, 1]))),
            "dual_mono_identical": bool(np.max(np.abs(stereo[:, 0] - stereo[:, 1])) == 0.0),
            # Approved mid layer and structured grit, both measured on mono.
            "energy_700_2500": float(
                np.mean([segments["high"]["energy_700_2500"], segments["redline"]["energy_700_2500"]])
            ),
            "energy_2500_7000": float(
                np.mean(
                    [segments["high"]["energy_2500_7000"], segments["redline"]["energy_2500_7000"]]
                )
            ),
            "energy_2500_7000_high": float(segments["high"]["energy_2500_7000"]),
            "energy_2500_7000_redline": float(segments["redline"]["energy_2500_7000"]),
        },
        "segments": segments,
        "v2": {**v2, "windows": len(v2_windows),
               "e_2_5": float(v2["engine_order_2_5_energy"]),
               "e_5": float(v2["engine_order_5_energy"]),
               "e_5_over_e_2_5_db": 10.0 * math.log10(max(float(v2["engine_order_5_energy"]), EPS) /
                                                        max(float(v2["engine_order_2_5_energy"]), EPS)),
               "channels": {**channel_v2, "mono": {k: v for k, v in v2.items() if k in ("mean", "rms", "ac_rms", "dc_ratio", "dc_to_ac_db")}}},
        "metadata": metadata,
        # Combustion-only: two voices from the same excitation, and the
        # modulation health of the upper band for fixed-RPM renders.
        "combustion_voices": analyze_combustion_stems(stem_dir)
        if stem_dir is not None
        else {"stems_present": False},
        "band_modulation": band_envelope_modulation(mono, sr),
        "diagnostics": diagnostics,
        "thresholds": {
            "idle_inaudible_dbfs": -45.0,
            "idle_silent_dbfs": -70.0,
            "noise_flatness": 0.35,
            "pure_tone_top_10_ratio": 0.92,
            "pure_tone_max_significant_orders": 2,
            "firing_error_percent": 8.0,
            "dual_mono_max_lr_diff": 0.0,
            "dc_ratio": 0.05,
            "half_block_leakage_db": 3.0,
            "single_partial_dominance_ratio": 0.5,
        },
    }
    arrays = {
        "stereo": stereo,
        "frequencies": frequencies,
        "times": times,
        "spectrum_db": 20.0 * np.log10(np.maximum(np.abs(spectrum), EPS)),
        "rpm": rpm,
        "frame_rms_dbfs": frame_rms_dbfs,
        "flatness": flatness,
        "top1": top1,
        "expected_firing": expected_firing,
        "observed_firing": observed_firing,
        "firing_error": firing_error,
    }
    return result, arrays


def plot_report(result: dict, arrays: dict[str, np.ndarray], output: Path) -> None:
    times = arrays["times"]
    figure, axes = plt.subplots(4, 1, figsize=(14, 13), constrained_layout=True)
    axes[0].plot(times, arrays["rpm"], label="RPM", color="tab:blue")
    rpm_axis = axes[0]
    level_axis = rpm_axis.twinx()
    level_axis.plot(times, arrays["frame_rms_dbfs"], label="RMS", color="tab:orange", alpha=0.85)
    rpm_axis.set_ylabel("RPM")
    level_axis.set_ylabel("RMS (dBFS)")
    rpm_axis.set_title("Rampa y nivel de salida")
    rpm_axis.grid(alpha=0.25)

    spectrum = arrays["spectrum_db"]
    frequencies = arrays["frequencies"]
    visible = frequencies <= 8000.0
    image = axes[1].pcolormesh(times, frequencies[visible], spectrum[visible], shading="auto", cmap="magma", vmin=-100, vmax=-15)
    axes[1].plot(times, arrays["expected_firing"], color="cyan", linewidth=1.2, label="RPM / 12")
    axes[1].set_ylim(0, 8000)
    axes[1].set_ylabel("Frecuencia (Hz)")
    axes[1].set_title("Espectrograma y frecuencia de encendido esperada")
    axes[1].legend(loc="upper left")
    figure.colorbar(image, ax=axes[1], label="dBFS")

    axes[2].plot(times, arrays["flatness"], label="Flatness", color="tab:green")
    axes[2].plot(times, arrays["top1"], label="Energía en bin dominante", color="tab:red")
    axes[2].axhline(0.35, color="tab:green", linestyle="--", alpha=0.6, label="Umbral ruido")
    axes[2].set_ylim(0, 1)
    axes[2].set_ylabel("Proporción")
    axes[2].set_title("Ruido frente a concentración tonal")
    axes[2].legend(loc="upper right")
    axes[2].grid(alpha=0.25)

    axes[3].plot(times, arrays["expected_firing"], label="Esperada", color="tab:blue")
    axes[3].plot(times, arrays["observed_firing"], label="Observada", color="tab:orange", alpha=0.8)
    axes[3].set_xlabel("Tiempo (s)")
    axes[3].set_ylabel("Frecuencia (Hz)")
    axes[3].set_title(
        "Seguimiento del encendido V10 — error mediano "
        f"{result['global']['median_firing_frequency_error_percent']:.1f}%"
    )
    axes[3].legend(loc="upper left")
    axes[3].grid(alpha=0.25)
    output.parent.mkdir(parents=True, exist_ok=True)
    figure.savefig(output, dpi=160)
    plt.close(figure)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("wav", type=Path)
    parser.add_argument("telemetry", type=Path)
    parser.add_argument("--json", required=True, type=Path)
    parser.add_argument("--plot", required=True, type=Path)
    parser.add_argument(
        "--stems",
        type=Path,
        default=None,
        help="stem directory for combustion-voice provenance checks",
    )
    args = parser.parse_args()
    result, arrays = analyze(args.wav, args.telemetry, args.stems)
    args.json.parent.mkdir(parents=True, exist_ok=True)
    args.json.write_text(json.dumps(result, indent=2, ensure_ascii=False), encoding="utf-8")
    plot_report(result, arrays, args.plot)
    print(json.dumps(result, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
