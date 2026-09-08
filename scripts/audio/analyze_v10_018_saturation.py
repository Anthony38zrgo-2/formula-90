"""Measure the isolated V10-018 load-saturation A/B renders."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import wave
from array import array
from pathlib import Path

import numpy as np


def read_wav(path: Path) -> tuple[int, np.ndarray]:
    with wave.open(str(path), "rb") as audio:
        if audio.getnchannels() != 1 or audio.getsampwidth() != 2:
            raise ValueError(f"expected mono PCM16: {path}")
        raw = array("h")
        raw.frombytes(audio.readframes(audio.getnframes()))
        values = np.asarray(raw, dtype=np.float32) / 32768.0
        return audio.getframerate(), values


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def rms(values: np.ndarray) -> float:
    return float(np.sqrt(np.mean(np.square(values, dtype=np.float64))))


def crest(values: np.ndarray) -> float:
    level = rms(values)
    return float(np.max(np.abs(values)) / level) if level > 0.0 else 0.0


def roughness_proxy(values: np.ndarray, sample_rate: int) -> float:
    """Envelope modulation proxy, not a perceptual loudness claim."""

    envelope = np.abs(values).astype(np.float64)
    width = max(3, int(round(sample_rate * 0.001)))
    kernel = np.ones(width, dtype=np.float64) / width
    smoothed = np.convolve(envelope, kernel, mode="same")
    level = rms(smoothed.astype(np.float32))
    return float(rms(np.diff(smoothed).astype(np.float32)) / level) if level > 0.0 else 0.0


def band_rms(values: np.ndarray, sample_rate: int, low_hz: float, high_hz: float) -> float:
    window = values * np.hanning(values.size)
    spectrum = np.fft.rfft(window)
    frequencies = np.fft.rfftfreq(values.size, 1.0 / sample_rate)
    selected = np.abs(spectrum[(frequencies >= low_hz) & (frequencies < high_hz)])
    return float(np.sqrt(np.mean(np.square(selected, dtype=np.float64)))) if selected.size else 0.0


def harmonic_orders(values: np.ndarray, sample_rate: int, rpm: float) -> dict[str, float]:
    window = values * np.hanning(values.size)
    spectrum = np.abs(np.fft.rfft(window))
    frequencies = np.fft.rfftfreq(window.size, 1.0 / sample_rate)
    fundamental = rpm / 120.0
    result: dict[str, float] = {}
    for order in range(1, 9):
        target = fundamental * order
        band = np.abs(frequencies - target) <= max(3.0, fundamental * 0.06)
        result[str(order)] = float(np.max(spectrum[band])) if np.any(band) else 0.0
    return result


def load_metadata(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def measure_variant(path: Path, rpm: float | None) -> dict:
    sample_rate, output = read_wav(path / "out.wav")
    warmup = min(sample_rate, output.size // 4)
    window = output[warmup:]
    stems: dict[str, dict[str, float | str]] = {}
    for stem_name in ["scene_mix", "low_mid_parallel", "load_saturation", "metallic_structure"]:
        stem_path = path / "stems" / f"{stem_name}.wav"
        if not stem_path.exists():
            continue
        stem_rate, stem = read_wav(stem_path)
        stem_window = stem[min(stem_rate, stem.size // 4) :]
        stems[stem_name] = {
            "rms": rms(stem_window),
            "peak": float(np.max(np.abs(stem_window))),
            "crest_factor": crest(stem_window),
            "low_mid_band_rms_80_500_hz": band_rms(stem_window, stem_rate, 80.0, 500.0),
            "roughness_proxy": roughness_proxy(stem_window, stem_rate),
            "sha256": sha256(stem_path),
        }
        if rpm is not None and stem_name in {"scene_mix", "load_saturation"}:
            stems[stem_name]["harmonic_orders"] = harmonic_orders(stem_window, stem_rate, rpm)
    metadata = load_metadata(path / "out.metadata.json")
    return {
        "sample_rate_hz": sample_rate,
        "frames": int(output.size),
        "duration_s": float(output.size / sample_rate),
        "scene_rms": rms(window),
        "scene_peak": float(np.max(np.abs(window))),
        "scene_crest_factor": crest(window),
        "scene_roughness_proxy": roughness_proxy(window, sample_rate),
        "scene_clipping_samples": int(np.count_nonzero(np.abs(window) >= 0.999)),
        "worst_limiter_reduction_db": float(metadata["worst_limiter_reduction_db"]),
        "stems": stems,
        "metadata": metadata,
    }


def compare_case(case: Path) -> dict:
    full_metadata = load_metadata(case / "full" / "out.metadata.json")
    rpm = float(full_metadata["rpm"]) if full_metadata["profile"] == "steady" else None
    full = measure_variant(case / "full", rpm)
    excised = measure_variant(case / "excised", rpm)
    _, full_output = read_wav(case / "full" / "out.wav")
    _, excised_output = read_wav(case / "excised" / "out.wav")
    count = min(full_output.size, excised_output.size)
    delta = full_output[:count] - excised_output[:count]
    full_level = rms(full_output[:count])
    return {
        "case": case.name,
        "inputs": {
            "rpm": float(full_metadata["rpm"]),
            "throttle": float(full_metadata["throttle"]),
            "load": float(full_metadata["load"]),
            "profile": full_metadata["profile"],
        },
        "full": full,
        "excised": excised,
        "paired_difference": {
            "rms": rms(delta),
            "peak": float(np.max(np.abs(delta))),
            "rms_over_full_rms": float(rms(delta) / full_level) if full_level else 0.0,
            "roughness_proxy": roughness_proxy(delta, int(full["sample_rate_hz"])),
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("root", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    cases = sorted(path for path in args.root.iterdir() if path.is_dir())
    if not cases:
        raise SystemExit(f"no V10-018 cases found under {args.root}")
    measurements = [compare_case(case) for case in cases]
    payload = {
        "schema_version": "v10-018-load-saturation-ab-v1",
        "root": str(args.root),
        "case_count": len(measurements),
        "comparison": "full corrected scene versus graph-excised load-dependent saturation; parallel compressor unchanged",
        "roughness_definition": "1 ms-smoothed absolute-envelope first-difference RMS divided by envelope RMS; diagnostic proxy only",
        "protection_definition": "scene-only PCM peak and renderer limiter metadata; no hidden makeup gain or master saturation retune",
        "human_listening_gate": "PENDING_HUMAN_LISTENING",
        "cases": measurements,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"cases": len(measurements), "output": str(args.output)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
