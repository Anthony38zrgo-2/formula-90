"""Measure the corrected V10-011 reference matrix without altering captures."""

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
        if raw.itemsize != 2:
            raise ValueError(f"unexpected PCM width: {path}")
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


def spectral_orders(values: np.ndarray, sample_rate: int, rpm: float) -> dict[str, float]:
    # Discard warmup and measure a stable 2 s window. The bin-local maxima make
    # this a structural order indicator, not a claim of perceptual realism.
    window = values[int(sample_rate): int(sample_rate * 3)]
    if window.size == 0:
        window = values
    window = window * np.hanning(window.size)
    spectrum = np.abs(np.fft.rfft(window))
    freqs = np.fft.rfftfreq(window.size, 1.0 / sample_rate)
    fundamental = rpm / 120.0
    output = {}
    for order in range(1, 11):
        target = fundamental * order
        if target >= sample_rate / 2:
            continue
        band = np.abs(freqs - target) <= max(3.0, fundamental * 0.08)
        output[str(order)] = float(np.max(spectrum[band])) if np.any(band) else 0.0
    return output


def measure_case(case: Path) -> dict:
    metadata = json.loads(case.joinpath("out.metadata.json").read_text(encoding="utf-8-sig"))
    sample_rate, output = read_wav(case / "out.wav")
    stems = {}
    for stem_name in ["combustion_source", "scene_mix", "sample_layer", "hybrid_mix", "master_pre_limiter", "master"]:
        stem = case / "stems" / f"{stem_name}.wav"
        if stem.exists():
            _, stem_values = read_wav(stem)
            stems[stem_name] = {
                "rms": rms(stem_values),
                "peak": float(np.max(np.abs(stem_values))),
                "sha256": sha256(stem),
            }
    source_rms = stems.get("combustion_source", {}).get("rms", 0.0)
    scene_rms = stems.get("scene_mix", {}).get("rms", 0.0)
    sample_rms = stems.get("sample_layer", {}).get("rms", 0.0)
    hybrid_rms = stems.get("hybrid_mix", {}).get("rms", 0.0)
    return {
        "case": case.name,
        "inputs": {
            "rpm": float(metadata["rpm"]),
            "throttle": float(metadata["throttle"]),
            "load": float(metadata["load"]),
        },
        "audio": {
            "sample_rate_hz": sample_rate,
            "frames": int(output.size),
            "duration_s": float(output.size / sample_rate),
            "rms": rms(output),
            "peak": float(np.max(np.abs(output))),
            "finite": bool(np.isfinite(output).all()),
        },
        "protection": {
            "worst_limiter_reduction_db": float(metadata["worst_limiter_reduction_db"]),
            "master_pre_limiter_peak": stems.get("master_pre_limiter", {}).get("peak", 0.0),
            "master_peak": stems.get("master", {}).get("peak", 0.0),
        },
        "direct_transitive_rms": {
            "combustion_source": source_rms,
            "scene_mix": scene_rms,
            "sample_layer": sample_rms,
            "hybrid_mix": hybrid_rms,
            "scene_to_hybrid_ratio": float(hybrid_rms / scene_rms) if scene_rms else 0.0,
            "sample_to_hybrid_ratio": float(sample_rms / hybrid_rms) if hybrid_rms else 0.0,
        },
        "spectral_order_indicator": spectral_orders(output, sample_rate, float(metadata["rpm"])),
        "stems": stems,
        "metadata": metadata,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("root", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    cases = sorted(path.parent for path in args.root.glob("r*_t*_l*/out.wav"))
    if len(cases) != 54:
        raise SystemExit(f"expected 54 corrected cases, found {len(cases)}")
    measurements = [measure_case(case) for case in cases]
    payload = {
        "schema_version": "v10-011-corrected-reference-measurements-v1",
        "reference_root": str(args.root),
        "case_count": len(measurements),
        "human_listening_gate": "PENDING_HUMAN_LISTENING",
        "historical_context": "E3/E4 reports are historical measurements and are not current rankings.",
        "measurement_contract": {
            "direct_transitive": "RMS and peak taps from the corrected renderer stem graph",
            "spectral_order": "bin-local FFT indicators around integer firing orders; structural only",
            "protection": "renderer master_pre_limiter/master peaks and metadata limiter reduction",
            "processing_cost": "see V10-004 runtime benchmark; no inferred game-wide percentage",
        },
        "cases": measurements,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"cases": len(measurements), "output": str(args.output)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
