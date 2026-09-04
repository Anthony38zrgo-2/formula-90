#!/usr/bin/env python3
"""Prepare steady V10 recordings for a phase-aware three-zone sample layer.

The source WAVs are read-only. For each input this tool writes a normalized
full-length signal, complementary tonal/residual stems, a 720-degree-aligned
loop of each signal, and JSON metadata intended for the Rust runtime.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import warnings
from dataclasses import asdict, dataclass
from pathlib import Path

import numpy as np
from scipy.io import wavfile

from analyze_v10_references import estimate_shaft_frequency, spectrum


@dataclass
class PreparedSample:
    schema_version: int
    source_path: str
    source_sha256: str
    sample_rate: int
    source_channels: int
    source_frames: int
    rpm_anchor: float
    rpm_source: str
    shaft_hz: float
    firing_order: float
    firing_hz: float
    firing_phase_degrees_at_loop_start: float
    phase_convention: str
    loop_start_source_sample: int
    loop_end_source_sample_exclusive: int
    loop_length_samples: int
    loop_engine_cycles_720: int
    loop_crossfade_samples: int
    loop_effective_rpm: float
    loop_seam_error_normalized: float
    source_dc: float
    source_rms_dbfs: float
    normalization_gain: float
    normalized_peak_dbfs: float
    tonal_energy_fraction: float
    files: dict[str, str]


def read_audio(path: Path, target_sample_rate: int | None = None) -> tuple[int, np.ndarray, int]:
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        sample_rate, raw = wavfile.read(path)
    channels = 1 if raw.ndim == 1 else raw.shape[1]
    if np.issubdtype(raw.dtype, np.integer):
        info = np.iinfo(raw.dtype)
        audio = raw.astype(np.float64) / float(max(abs(info.min), info.max))
    elif np.issubdtype(raw.dtype, np.floating):
        audio = raw.astype(np.float64)
    else:
        raise ValueError(f"unsupported WAV dtype {raw.dtype}: {path}")
    mono = audio if audio.ndim == 1 else np.mean(audio, axis=1)
    if target_sample_rate is not None and target_sample_rate != sample_rate:
        import math
        import scipy.signal as signal
        g = math.gcd(int(sample_rate), int(target_sample_rate))
        mono = signal.resample_poly(mono, target_sample_rate // g, sample_rate // g)
        sample_rate = target_sample_rate
    if len(mono) < sample_rate // 2:
        raise ValueError(f"sample must be at least 0.5 seconds long: {path}")
    if not np.all(np.isfinite(mono)):
        raise ValueError(f"sample contains non-finite values: {path}")
    return int(sample_rate), mono, channels


def parse_rpm_assignments(values: list[str]) -> dict[str, float]:
    assignments: dict[str, float] = {}
    for value in values:
        if "=" not in value:
            raise ValueError(f"invalid --rpm assignment {value!r}; expected FILE=RPM")
        name, raw_rpm = value.rsplit("=", 1)
        rpm = float(raw_rpm)
        if not name or not math.isfinite(rpm) or rpm <= 0.0:
            raise ValueError(f"invalid --rpm assignment {value!r}")
        assignments[Path(name).name.casefold()] = rpm
        assignments[Path(name).stem.casefold()] = rpm
    return assignments


def resolve_rpm(path: Path, audio: np.ndarray, sample_rate: int, assignments: dict[str, float]):
    for key in (path.name.casefold(), path.stem.casefold()):
        if key in assignments:
            return assignments[key], "explicit"
    freq, power = spectrum(audio - np.mean(audio), sample_rate)
    shaft_hz, _ = estimate_shaft_frequency(freq, power)
    return shaft_hz * 60.0, "estimated"


def tonal_residual(audio: np.ndarray, sample_rate: int, shaft_hz: float) -> tuple[np.ndarray, np.ndarray]:
    """Split stationary audio with complementary soft masks at half crank orders."""
    transformed = np.fft.rfft(audio)
    frequencies = np.fft.rfftfreq(len(audio), 1.0 / sample_rate)
    mask = np.zeros_like(frequencies)
    max_order = int((min(12_000.0, frequencies[-1]) / shaft_hz) * 2.0)
    half_width = max(3.0, shaft_hz * 0.022)
    for half_order in range(1, max_order + 1):
        center = shaft_hz * half_order * 0.5
        distance = np.abs(frequencies - center)
        local = distance < half_width
        mask[local] = np.maximum(
            mask[local], 0.5 + 0.5 * np.cos(np.pi * distance[local] / half_width)
        )
    # Do not classify sub-audible DC or ultrasonic bins as engine orders.
    mask[(frequencies < 20.0) | (frequencies > 12_000.0)] = 0.0
    tonal = np.fft.irfft(transformed * mask, n=len(audio))
    residual = audio - tonal
    return tonal, residual


def normalized_seam_error(audio: np.ndarray, start: int, length: int, window: int) -> float:
    left = audio[start : start + window]
    # Compare the signal that will be heard after wrapping (at `start`) with
    # the signal that would naturally follow the exclusive loop end. Comparing
    # against the final internal window would measure a one-window phase offset.
    right = audio[start + length : start + length + window]
    scale = float(np.sqrt(np.mean(np.concatenate((left, right)) ** 2))) + 1e-12
    value_error = float(np.sqrt(np.mean((left - right) ** 2))) / scale
    slope_error = float(np.sqrt(np.mean((np.diff(left) - np.diff(right)) ** 2))) / scale
    local = audio[start + length - window : start + length + window]
    typical_delta = float(np.sqrt(np.mean(np.diff(local) ** 2))) + 1e-12
    boundary_jump = abs(float(audio[start + length] - audio[start + length - 1])) / typical_delta
    return value_error + 0.35 * slope_error + 0.65 * boundary_jump


def find_engine_loop(
    audio: np.ndarray,
    sample_rate: int,
    shaft_hz: float,
    requested_cycles: int | None = None,
):
    period_720 = sample_rate * 2.0 / shaft_hz
    window = min(768, max(128, round(period_720 * 0.75)))
    margin = max(window, round(sample_rate * 0.08))
    best: tuple[float, int, int, int] | None = None
    cycle_counts = [requested_cycles] if requested_cycles is not None else range(4, 17)
    for cycles in cycle_counts:
        ideal_length = period_720 * cycles
        for length in range(round(ideal_length) - 2, round(ideal_length) + 3):
            last_start = len(audio) - margin - length - window
            if last_start <= margin or length <= window * 2:
                continue
            step = max(1, round(period_720 / 24.0))
            for start in range(margin, last_start + 1, step):
                score = normalized_seam_error(audio, start, length, window)
                if best is None or score < best[0]:
                    best = (score, start, length, cycles)
    if best is None:
        raise ValueError("audio is too short to locate a 720-degree loop")
    return best


def make_crossfaded_loop(audio: np.ndarray, start: int, length: int, fade: int) -> np.ndarray:
    """Crossfade the natural continuation into the head without changing loop length."""
    head = audio[start : start + fade]
    continuation = audio[start + length : start + length + fade]
    theta = np.linspace(0.0, np.pi * 0.5, fade, endpoint=False)
    # Squared equal-power weights are complementary. This avoids the +3 dB
    # bulge that plain sin/cos creates when both sides are strongly correlated.
    blended = continuation * np.cos(theta) ** 2 + head * np.sin(theta) ** 2
    return np.concatenate((blended, audio[start + fade : start + length]))


def circular_boundary_error(audio: np.ndarray) -> float:
    scale = float(np.sqrt(np.mean(audio**2))) + 1e-12
    value_error = abs(float(audio[0] - audio[-1])) / scale
    slope_error = abs(float((audio[1] - audio[0]) - (audio[-1] - audio[-2]))) / scale
    return value_error + 0.35 * slope_error


def firing_phase_degrees(audio: np.ndarray, sample_rate: int, start: int, length: int, firing_hz: float):
    segment = audio[start : start + length]
    time = np.arange(length, dtype=np.float64) / sample_rate
    coefficient = np.sum(segment * np.exp(-2j * np.pi * firing_hz * time))
    return float(np.degrees(np.angle(coefficient)) % 360.0)


def write_pcm16(path: Path, sample_rate: int, audio: np.ndarray) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    pcm = np.round(np.clip(audio, -1.0, 1.0) * 32767.0).astype(np.int16)
    wavfile.write(path, sample_rate, pcm)


def prepare(
    path: Path,
    output_dir: Path,
    rpm_assignments: dict[str, float],
    target_peak_dbfs: float,
    loop_cycles: int | None = None,
    target_sample_rate: int | None = None,
):
    sample_rate, source, channels = read_audio(path, target_sample_rate)
    source_dc = float(np.mean(source))
    centered = source - source_dc
    rpm, rpm_source = resolve_rpm(path, centered, sample_rate, rpm_assignments)
    shaft_hz = rpm / 60.0
    firing_hz = shaft_hz * 5.0
    _, loop_start, loop_length, loop_cycles = find_engine_loop(
        centered, sample_rate, shaft_hz, loop_cycles
    )
    tonal, residual = tonal_residual(centered, sample_rate, shaft_hz)

    target_peak = 10.0 ** (target_peak_dbfs / 20.0)
    peak = float(np.max(np.abs(centered)))
    gain = target_peak / max(peak, 1e-12)
    normalized = centered * gain
    tonal *= gain
    residual *= gain
    end = loop_start + loop_length
    crossfade = min(768, max(128, round(sample_rate * 0.008)))
    normalized_loop = make_crossfaded_loop(normalized, loop_start, loop_length, crossfade)
    tonal_loop = make_crossfaded_loop(tonal, loop_start, loop_length, crossfade)
    residual_loop = make_crossfaded_loop(residual, loop_start, loop_length, crossfade)
    seam_error = circular_boundary_error(normalized_loop)
    slug = path.stem
    files = {
        "normalized": f"{slug}.normalized.wav",
        "tonal": f"{slug}.tonal.wav",
        "residual": f"{slug}.residual.wav",
        "loop": f"{slug}.loop.wav",
        "loop_tonal": f"{slug}.loop.tonal.wav",
        "loop_residual": f"{slug}.loop.residual.wav",
    }
    write_pcm16(output_dir / files["normalized"], sample_rate, normalized)
    write_pcm16(output_dir / files["tonal"], sample_rate, tonal)
    write_pcm16(output_dir / files["residual"], sample_rate, residual)
    write_pcm16(output_dir / files["loop"], sample_rate, normalized_loop)
    write_pcm16(output_dir / files["loop_tonal"], sample_rate, tonal_loop)
    write_pcm16(output_dir / files["loop_residual"], sample_rate, residual_loop)

    source_rms = float(np.sqrt(np.mean(centered**2)))
    tonal_fraction = float(np.sum(tonal**2) / (np.sum(normalized**2) + 1e-30))
    metadata = PreparedSample(
        schema_version=1,
        source_path=str(path.resolve()),
        source_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
        sample_rate=sample_rate,
        source_channels=channels,
        source_frames=len(source),
        rpm_anchor=rpm,
        rpm_source=rpm_source,
        shaft_hz=shaft_hz,
        firing_order=5.0,
        firing_hz=firing_hz,
        firing_phase_degrees_at_loop_start=firing_phase_degrees(
            normalized_loop, sample_rate, 0, loop_length, firing_hz
        ),
        phase_convention="angle of sum(x[n] * exp(-j*2*pi*firing_hz*n/fs)) at loop start",
        loop_start_source_sample=loop_start,
        loop_end_source_sample_exclusive=end,
        loop_length_samples=loop_length,
        loop_engine_cycles_720=loop_cycles,
        loop_crossfade_samples=crossfade,
        loop_effective_rpm=sample_rate * 120.0 * loop_cycles / loop_length,
        loop_seam_error_normalized=seam_error,
        source_dc=source_dc,
        source_rms_dbfs=20.0 * math.log10(max(source_rms, 1e-30)),
        normalization_gain=gain,
        normalized_peak_dbfs=target_peak_dbfs,
        tonal_energy_fraction=tonal_fraction,
        files=files,
    )
    metadata_path = output_dir / f"{slug}.sample-layer.json"
    metadata_path.write_text(json.dumps(asdict(metadata), indent=2), encoding="utf-8")
    return metadata_path, metadata


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inputs", nargs="+", type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument(
        "--rpm",
        action="append",
        default=[],
        metavar="FILE=RPM",
        help="Anchor RPM for one input; repeat per file. Unspecified inputs are estimated.",
    )
    parser.add_argument("--target-peak-dbfs", type=float, default=-1.0)
    parser.add_argument(
        "--loop-cycles",
        type=int,
        help="Require an exact number of complete 720-degree engine cycles per loop.",
    )
    parser.add_argument(
        "--sample-rate",
        type=int,
        help="Resample input audio to this target sample rate (e.g. 44100).",
    )
    args = parser.parse_args()
    if not math.isfinite(args.target_peak_dbfs) or not (-24.0 <= args.target_peak_dbfs <= -0.1):
        parser.error("--target-peak-dbfs must be finite and inside -24..-0.1")
    if args.loop_cycles is not None and args.loop_cycles <= 0:
        parser.error("--loop-cycles must be a positive integer")
    if args.sample_rate is not None and args.sample_rate <= 0:
        parser.error("--sample-rate must be a positive integer")
    try:
        rpm_assignments = parse_rpm_assignments(args.rpm)
    except ValueError as error:
        parser.error(str(error))
    missing = [str(path) for path in args.inputs if not path.is_file()]
    if missing:
        parser.error(f"input files not found: {', '.join(missing)}")
    args.output_dir.mkdir(parents=True, exist_ok=True)
    manifest = []
    for path in args.inputs:
        metadata_path, metadata = prepare(
            path,
            args.output_dir,
            rpm_assignments,
            args.target_peak_dbfs,
            args.loop_cycles,
            args.sample_rate,
        )
        manifest.append(asdict(metadata))
        print(
            f"{path.name}: rpm={metadata.rpm_anchor:.2f} ({metadata.rpm_source}) "
            f"loop={metadata.loop_length_samples} samples/{metadata.loop_engine_cycles_720} cycles "
            f"phase={metadata.firing_phase_degrees_at_loop_start:.2f}deg "
            f"metadata={metadata_path}"
        )
    (args.output_dir / "sample-layer-manifest.json").write_text(
        json.dumps({"schema_version": 1, "samples": manifest}, indent=2), encoding="utf-8"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
