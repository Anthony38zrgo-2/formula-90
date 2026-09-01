#!/usr/bin/env python3
"""Reverse-engineer steady V10 reference loops without third-party audio packages.

Uses only NumPy, SciPy and Matplotlib. It never modifies or copies the source
audio. Outputs derived metrics, order spectra and figures for engineering use.
"""

from __future__ import annotations

import argparse
import json
import math
import struct
import warnings
from dataclasses import asdict, dataclass
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
from scipy.io import wavfile
from scipy.signal import find_peaks, get_window, stft, welch


@dataclass
class AudioMetrics:
    name: str
    path: str
    sample_rate: int
    channels: int
    frames: int
    duration_s: float
    dtype: str
    peak: float
    rms: float
    rms_dbfs: float
    crest_db: float
    dc: float
    clipping_fraction: float
    shaft_hz: float
    implied_rpm: float
    firing_hz: float
    firing_order: float
    firing_energy_fraction: float
    harmonic_energy_fraction: float
    spectral_centroid_hz: float
    spectral_rolloff_85_hz: float
    spectral_flatness: float
    band_20_80: float
    band_80_250: float
    band_250_600: float
    band_600_2500: float
    band_2500_7000: float
    band_7000_16000: float
    shaft_cycle_corr: float
    engine_cycle_720_corr: float
    adjacent_engine_cycle_corr_mean: float
    adjacent_engine_cycle_corr_std: float
    envelope_modulation_hz: float
    envelope_modulation_depth: float
    seam_value_jump: float
    seam_slope_jump: float
    best_seam_correlation: float
    best_seam_lag_samples: int
    riff_chunks: list[dict]
    sampler_loops: list[dict]
    top_spectral_peaks: list[dict]


def read_riff_chunks(path: Path) -> tuple[list[dict], list[dict]]:
    chunks: list[dict] = []
    loops: list[dict] = []
    with path.open("rb") as fh:
        header = fh.read(12)
        if len(header) != 12 or header[:4] not in (b"RIFF", b"RF64") or header[8:] != b"WAVE":
            raise ValueError(f"not a RIFF/WAVE file: {path}")
        while True:
            chunk_header = fh.read(8)
            if len(chunk_header) < 8:
                break
            chunk_id, size = struct.unpack("<4sI", chunk_header)
            offset = fh.tell()
            chunks.append({"id": chunk_id.decode("latin1"), "size": size, "offset": offset})
            payload = fh.read(size)
            if chunk_id == b"smpl" and len(payload) >= 36:
                fields = struct.unpack_from("<9I", payload, 0)
                loop_count = fields[7]
                for index in range(loop_count):
                    base = 36 + index * 24
                    if base + 24 <= len(payload):
                        cue_id, loop_type, start, end, fraction, play_count = struct.unpack_from(
                            "<6I", payload, base
                        )
                        loops.append(
                            {
                                "cue_id": cue_id,
                                "type": loop_type,
                                "start": start,
                                "end_inclusive": end,
                                "length_samples": end - start + 1,
                                "fraction": fraction,
                                "play_count": play_count,
                                "midi_unity_note": fields[3],
                                "sample_period_ns": fields[2],
                            }
                        )
            if size & 1:
                fh.seek(1, 1)
    return chunks, loops


def read_audio(path: Path) -> tuple[int, np.ndarray, str, int]:
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        sample_rate, raw = wavfile.read(path)
    channels = 1 if raw.ndim == 1 else raw.shape[1]
    dtype_name = str(raw.dtype)
    if np.issubdtype(raw.dtype, np.integer):
        info = np.iinfo(raw.dtype)
        scale = float(max(abs(info.min), info.max))
        audio = raw.astype(np.float64) / scale
    elif np.issubdtype(raw.dtype, np.floating):
        audio = raw.astype(np.float64)
    else:
        raise ValueError(f"unsupported WAV dtype {raw.dtype}")
    mono = audio if audio.ndim == 1 else np.mean(audio, axis=1)
    return int(sample_rate), mono, dtype_name, channels


def spectrum(audio: np.ndarray, sample_rate: int) -> tuple[np.ndarray, np.ndarray]:
    nperseg = min(32768, len(audio))
    if nperseg < 2048:
        raise ValueError("audio is too short for reference analysis")
    freq, power = welch(
        audio - np.mean(audio),
        sample_rate,
        window="hann",
        nperseg=nperseg,
        noverlap=nperseg * 3 // 4,
        scaling="spectrum",
    )
    return freq, power


def estimate_shaft_frequency(freq: np.ndarray, power: np.ndarray) -> tuple[float, float]:
    # A 1990s V10 loop should encode shaft order between roughly 80 and 250 Hz.
    # Score a dense grid by energy at integer crank orders, emphasizing order 5
    # (one V10 firing family) without allowing it to determine the answer alone.
    # Cover 4,800--19,200 rpm. The highest reference layer is calibrated near
    # 15,538 rpm, so capping this search at 250 Hz can mistake its 0.5 order
    # for the crank fundamental.
    candidates = np.arange(80.0, 320.0, 0.025)
    floor = float(np.median(power[(freq >= 40.0) & (freq <= 12000.0)])) + 1e-30
    scores = np.zeros_like(candidates)
    nyquist = freq[-1]
    for order in range(1, 41):
        target = candidates * order
        valid = target < min(nyquist, 12000.0)
        sampled = np.interp(target[valid], freq, power)
        weight = (2.4 if order % 5 == 0 else 1.0) / math.sqrt(order)
        scores[valid] += weight * np.log1p(sampled / floor)
    best = int(np.argmax(scores))
    return float(candidates[best]), float(scores[best])


def band_fraction(freq: np.ndarray, power: np.ndarray, lo: float, hi: float) -> float:
    useful = power[freq >= 20.0]
    total = float(np.sum(useful)) + 1e-30
    return float(np.sum(power[(freq >= lo) & (freq < hi)]) / total)


def periodic_correlation(audio: np.ndarray, period_samples: int) -> float:
    if period_samples <= 1 or len(audio) <= period_samples * 4:
        return 0.0
    a = audio[period_samples:-period_samples] - np.mean(audio[period_samples:-period_samples])
    b = audio[2 * period_samples :] - np.mean(audio[2 * period_samples :])
    n = min(len(a), len(b))
    denom = float(np.linalg.norm(a[:n]) * np.linalg.norm(b[:n])) + 1e-30
    return float(np.dot(a[:n], b[:n]) / denom)


def adjacent_cycle_stats(audio: np.ndarray, period_samples: int) -> tuple[float, float]:
    if period_samples < 8:
        return 0.0, 0.0
    start = min(len(audio) // 10, period_samples * 4)
    usable = audio[start : start + ((len(audio) - start) // period_samples) * period_samples]
    if len(usable) < period_samples * 3:
        return 0.0, 0.0
    cycles = usable.reshape(-1, period_samples)
    correlations = []
    for left, right in zip(cycles[:-1], cycles[1:]):
        left = left - np.mean(left)
        right = right - np.mean(right)
        denom = float(np.linalg.norm(left) * np.linalg.norm(right)) + 1e-30
        correlations.append(float(np.dot(left, right) / denom))
    return float(np.mean(correlations)), float(np.std(correlations))


def envelope_modulation(audio: np.ndarray, sample_rate: int) -> tuple[float, float]:
    hop = 128
    frame = 512
    if len(audio) < frame * 2:
        return 0.0, 0.0
    rms = np.array(
        [np.sqrt(np.mean(audio[i : i + frame] ** 2)) for i in range(0, len(audio) - frame, hop)]
    )
    rms -= np.mean(rms)
    env_rate = sample_rate / hop
    freq, power = welch(rms, env_rate, nperseg=min(1024, len(rms)), scaling="spectrum")
    mask = (freq >= 0.5) & (freq <= 100.0)
    if not np.any(mask):
        return 0.0, 0.0
    local = np.argmax(power[mask])
    index = np.flatnonzero(mask)[local]
    depth = float(np.sqrt(power[index]) / (np.mean(np.abs(rms)) + 1e-30))
    return float(freq[index]), depth


def seam_metrics(audio: np.ndarray) -> tuple[float, float, float, int]:
    value_jump = float(abs(audio[0] - audio[-1]))
    slope_jump = float(abs((audio[1] - audio[0]) - (audio[-1] - audio[-2])))
    window = min(8192, len(audio) // 4)
    left = audio[:window] - np.mean(audio[:window])
    right = audio[-window:] - np.mean(audio[-window:])
    best_corr = -1.0
    best_lag = 0
    for lag in range(-512, 513):
        if lag < 0:
            a, b = left[-lag:], right[: window + lag]
        elif lag > 0:
            a, b = left[: window - lag], right[lag:]
        else:
            a, b = left, right
        denom = float(np.linalg.norm(a) * np.linalg.norm(b)) + 1e-30
        corr = float(np.dot(a, b) / denom)
        if corr > best_corr:
            best_corr, best_lag = corr, lag
    return value_jump, slope_jump, best_corr, best_lag


def analyze(
    path: Path, forced_shaft_hz: float | None = None
) -> tuple[AudioMetrics, np.ndarray, np.ndarray, np.ndarray, np.ndarray]:
    sample_rate, audio, dtype_name, channels = read_audio(path)
    chunks, loops = read_riff_chunks(path)
    centered = audio - np.mean(audio)
    freq, power = spectrum(centered, sample_rate)
    shaft_hz = forced_shaft_hz
    if shaft_hz is None:
        shaft_hz, _ = estimate_shaft_frequency(freq, power)
    firing_hz = shaft_hz * 5.0
    resolution = freq[1] - freq[0]
    firing_mask = np.abs(freq - firing_hz) <= max(1.5 * resolution, 3.0)
    useful = freq >= 20.0
    total = float(np.sum(power[useful])) + 1e-30
    firing_fraction = float(np.sum(power[firing_mask]) / total)

    harmonic_mask = np.zeros_like(freq, dtype=bool)
    max_order = int(min(60, freq[-1] / shaft_hz))
    order_rows = []
    for order in np.arange(0.5, max_order + 0.5, 0.5):
        target = shaft_hz * order
        mask = np.abs(freq - target) <= max(1.5 * resolution, shaft_hz * 0.018)
        energy = float(np.sum(power[mask]))
        order_rows.append((float(order), target, energy / total))
        if float(order).is_integer():
            harmonic_mask |= mask
    harmonic_fraction = float(np.sum(power[harmonic_mask]) / total)

    weighted = power[useful]
    useful_freq = freq[useful]
    centroid = float(np.sum(useful_freq * weighted) / (np.sum(weighted) + 1e-30))
    cumulative = np.cumsum(weighted)
    rolloff = float(useful_freq[np.searchsorted(cumulative, cumulative[-1] * 0.85)])
    flatness = float(np.exp(np.mean(np.log(weighted + 1e-30))) / (np.mean(weighted) + 1e-30))

    db = 10.0 * np.log10(power + 1e-30)
    peak_indices, _ = find_peaks(db, prominence=3.0)
    top_indices = peak_indices[np.argsort(power[peak_indices])[-20:]][::-1]
    peaks = [
        {
            "frequency_hz": float(freq[index]),
            "order": float(freq[index] / shaft_hz),
            "db": float(db[index]),
        }
        for index in top_indices
    ]

    shaft_period = max(1, round(sample_rate / shaft_hz))
    engine_period = max(1, round(sample_rate / (shaft_hz / 2.0)))
    adjacent_mean, adjacent_std = adjacent_cycle_stats(centered, engine_period)
    modulation_hz, modulation_depth = envelope_modulation(centered, sample_rate)
    seam_value, seam_slope, seam_corr, seam_lag = seam_metrics(centered)
    peak = float(np.max(np.abs(audio)))
    rms = float(np.sqrt(np.mean(audio**2)))
    metrics = AudioMetrics(
        name=path.name,
        path=str(path),
        sample_rate=sample_rate,
        channels=channels,
        frames=len(audio),
        duration_s=len(audio) / sample_rate,
        dtype=dtype_name,
        peak=peak,
        rms=rms,
        rms_dbfs=20.0 * math.log10(max(rms, 1e-30)),
        crest_db=20.0 * math.log10(max(peak, 1e-30) / max(rms, 1e-30)),
        dc=float(np.mean(audio)),
        clipping_fraction=float(np.mean(np.abs(audio) >= 0.999)),
        shaft_hz=shaft_hz,
        implied_rpm=shaft_hz * 60.0,
        firing_hz=firing_hz,
        firing_order=5.0,
        firing_energy_fraction=firing_fraction,
        harmonic_energy_fraction=harmonic_fraction,
        spectral_centroid_hz=centroid,
        spectral_rolloff_85_hz=rolloff,
        spectral_flatness=flatness,
        band_20_80=band_fraction(freq, power, 20.0, 80.0),
        band_80_250=band_fraction(freq, power, 80.0, 250.0),
        band_250_600=band_fraction(freq, power, 250.0, 600.0),
        band_600_2500=band_fraction(freq, power, 600.0, 2500.0),
        band_2500_7000=band_fraction(freq, power, 2500.0, 7000.0),
        band_7000_16000=band_fraction(freq, power, 7000.0, min(16000.0, freq[-1])),
        shaft_cycle_corr=periodic_correlation(centered, shaft_period),
        engine_cycle_720_corr=periodic_correlation(centered, engine_period),
        adjacent_engine_cycle_corr_mean=adjacent_mean,
        adjacent_engine_cycle_corr_std=adjacent_std,
        envelope_modulation_hz=modulation_hz,
        envelope_modulation_depth=modulation_depth,
        seam_value_jump=seam_value,
        seam_slope_jump=seam_slope,
        best_seam_correlation=seam_corr,
        best_seam_lag_samples=seam_lag,
        riff_chunks=chunks,
        sampler_loops=loops,
        top_spectral_peaks=peaks,
    )
    return metrics, freq, power, np.asarray(order_rows), centered


def plot_overview(results, output: Path) -> None:
    rows = len(results)
    fig, axes = plt.subplots(rows, 3, figsize=(15, 3.8 * rows), constrained_layout=True)
    if rows == 1:
        axes = np.asarray([axes])
    for row, (metrics, freq, power, _, audio) in enumerate(results):
        sr = metrics.sample_rate
        time = np.arange(len(audio)) / sr
        stride = max(1, len(audio) // 5000)
        axes[row, 0].plot(time[::stride], audio[::stride], linewidth=0.55)
        axes[row, 0].set(title=metrics.name, xlabel="Time (s)", ylabel="Amplitude")
        f_stft, t_stft, z = stft(audio, sr, window="hann", nperseg=4096, noverlap=3584)
        keep = f_stft <= 10000
        axes[row, 1].pcolormesh(
            t_stft,
            f_stft[keep],
            20 * np.log10(np.abs(z[keep]) + 1e-7),
            shading="auto",
            vmin=-95,
            vmax=-15,
            cmap="magma",
        )
        axes[row, 1].set(xlabel="Time (s)", ylabel="Frequency (Hz)", ylim=(0, 10000))
        axes[row, 2].semilogx(freq[1:], 10 * np.log10(power[1:] + 1e-30), linewidth=0.9)
        axes[row, 2].axvline(metrics.firing_hz, color="tab:red", linewidth=0.8, linestyle="--")
        axes[row, 2].set(xlabel="Frequency (Hz)", ylabel="Power (dB)", xlim=(20, 20000))
        axes[row, 2].grid(True, which="both", alpha=0.2)
    fig.savefig(output, dpi=150)
    plt.close(fig)


def plot_orders(results, output: Path) -> None:
    fig, ax = plt.subplots(figsize=(13, 6), constrained_layout=True)
    for metrics, _, _, orders, _ in results:
        energy = orders[:, 2]
        db = 10 * np.log10(energy / (np.max(energy) + 1e-30) + 1e-12)
        ax.plot(orders[:, 0], db, marker=".", markersize=3, linewidth=1, label=metrics.name)
    ax.axvline(5.0, color="black", linewidth=0.8, linestyle="--", label="V10 firing order 5")
    ax.set(xlabel="Crank order", ylabel="Relative order energy (dB)", xlim=(0.5, 40), ylim=(-70, 2))
    ax.grid(True, alpha=0.25)
    ax.legend()
    fig.savefig(output, dpi=160)
    plt.close(fig)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("inputs", nargs="+", type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument(
        "--forced-rpm",
        type=float,
        help="Known tuning RPM; resolves half-order ambiguity in layered game samples.",
    )
    args = parser.parse_args()
    args.output_dir.mkdir(parents=True, exist_ok=True)
    forced_shaft_hz = None if args.forced_rpm is None else args.forced_rpm / 60.0
    results = [analyze(path, forced_shaft_hz) for path in args.inputs]
    payload = {metrics.name: asdict(metrics) for metrics, *_ in results}
    (args.output_dir / "reference_metrics.json").write_text(
        json.dumps(payload, indent=2), encoding="utf-8"
    )
    with (args.output_dir / "order_spectra.csv").open("w", encoding="utf-8") as fh:
        fh.write("sample,shaft_hz,implied_rpm,order,frequency_hz,energy_fraction\n")
        for metrics, _, _, orders, _ in results:
            for order, frequency, energy in orders:
                fh.write(
                    f"{metrics.name},{metrics.shaft_hz:.6f},{metrics.implied_rpm:.3f},"
                    f"{order:.1f},{frequency:.6f},{energy:.12g}\n"
                )
    plot_overview(results, args.output_dir / "reference_overview.png")
    plot_orders(results, args.output_dir / "reference_order_spectra.png")
    for metrics, *_ in results:
        print(
            f"{metrics.name}: shaft={metrics.shaft_hz:.3f}Hz "
            f"rpm={metrics.implied_rpm:.1f} firing={metrics.firing_hz:.3f}Hz "
            f"rms={metrics.rms_dbfs:.2f}dBFS crest={metrics.crest_db:.2f}dB "
            f"firing_share={metrics.firing_energy_fraction:.3f} "
            f"harmonic_share={metrics.harmonic_energy_fraction:.3f}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
