#!/usr/bin/env python3
"""Generate the original formula-90s V10 prototype sample bank offline."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import random
import struct
import wave
from pathlib import Path

import yaml

LAYER_MULTIPLIERS = {"idle": 1.0, "low": 1.7, "mid": 2.7, "high": 3.9, "redline": 5.1}


def _remove_dc(samples: list[float]) -> list[float]:
    mean = sum(samples) / max(1, len(samples))
    return [value - mean for value in samples]


def _lowpass(samples: list[float], cutoff: float, sample_rate: int) -> list[float]:
    alpha = 1.0 - math.exp(-2.0 * math.pi * cutoff / sample_rate)
    state = 0.0
    output: list[float] = []
    for value in samples:
        state += alpha * (value - state)
        output.append(state)
    return output


def _normalize(samples: list[float], peak: float) -> list[float]:
    maximum = max((abs(value) for value in samples), default=1.0)
    scale = peak / maximum if maximum > 1e-12 else 1.0
    return [max(-0.999, min(0.999, value * scale)) for value in samples]


def _fade_edges(samples: list[float], fade_frames: int) -> None:
    count = min(fade_frames, len(samples) // 2)
    for index in range(count):
        gain = index / max(1, count - 1)
        samples[index] *= gain
        samples[-1 - index] *= gain


def synthesize_layer(config: dict[str, object], multiplier: float, seed_offset: int) -> list[float]:
    sample_rate = int(config["sample_rate"])
    count = int(sample_rate * float(config["duration_seconds"]))
    base = float(config["conceptual_base_frequency"]) * multiplier
    harmonics = [float(value) for value in str(config["harmonics"]).split(",")]
    rng = random.Random(int(config["seed"]) + seed_offset)
    samples: list[float] = []
    phases = [rng.random() * math.tau for _ in harmonics]
    for index in range(count):
        time = index / sample_rate
        flutter = 1.0 + 0.006 * math.sin(math.tau * 3.1 * time)
        value = 0.0
        for harmonic_index, gain in enumerate(harmonics, start=1):
            value += gain * math.sin(math.tau * base * harmonic_index * flutter * time + phases[harmonic_index - 1])
        pulse = math.sin(math.tau * base * int(config["pulse_density"]) * 0.1 * time)
        value += 0.08 * math.copysign(abs(pulse) ** 4, pulse)
        value += rng.uniform(-1.0, 1.0) * float(config["noise_gain"])
        samples.append(math.tanh(value * float(config["saturation"])))
    samples = _lowpass(_remove_dc(samples), float(config["lowpass_hz"]), sample_rate)
    samples = _normalize(samples, float(config["normalization_peak"]))
    _fade_edges(samples, int(float(config["fade_seconds"]) * sample_rate))
    return samples


def synthesize_shift(config: dict[str, object], upward: bool) -> list[float]:
    sample_rate = int(config["sample_rate"])
    count = int(sample_rate * float(config["gear_duration_seconds"]))
    rng = random.Random(int(config["seed"]) + (700 if upward else 701))
    samples: list[float] = []
    for index in range(count):
        position = index / max(1, count - 1)
        envelope = math.sin(math.pi * position) ** 2
        frequency = (240.0 + 190.0 * position) if upward else (420.0 - 230.0 * position)
        tone = math.sin(math.tau * frequency * index / sample_rate)
        noise = rng.uniform(-1.0, 1.0)
        samples.append(envelope * (0.55 * tone + 0.18 * noise))
    return _normalize(_remove_dc(samples), 0.68)


def write_wav(path: Path, samples: list[float], sample_rate: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = b"".join(struct.pack("<h", round(max(-1.0, min(1.0, value)) * 32767)) for value in samples)
    with wave.open(str(path), "wb") as output:
        output.setnchannels(1)
        output.setsampwidth(2)
        output.setframerate(sample_rate)
        output.writeframes(payload)


def generate(config_path: Path, output_directory: Path) -> dict[str, object]:
    config = yaml.safe_load(config_path.read_text(encoding="utf-8"))
    sample_rate = int(config["sample_rate"])
    files: dict[str, dict[str, object]] = {}
    for offset, (name, multiplier) in enumerate(LAYER_MULTIPLIERS.items()):
        path = output_directory / f"engine_{name}.wav"
        write_wav(path, synthesize_layer(config, multiplier, offset), sample_rate)
        files[path.name] = {"sha256": hashlib.sha256(path.read_bytes()).hexdigest(), "loop": True}
    for name, upward in (("gear_up", True), ("gear_down", False)):
        path = output_directory / f"{name}.wav"
        write_wav(path, synthesize_shift(config, upward), sample_rate)
        files[path.name] = {"sha256": hashlib.sha256(path.read_bytes()).hexdigest(), "loop": False}
    metadata = {
        "schema_version": 1,
        "generator": "formula90s offline synthesizer 1.0",
        "bank_name": config["bank_name"],
        "sample_rate": sample_rate,
        "channels": 1,
        "pcm_bits": 16,
        "seed": int(config["seed"]),
        "loop_start_seconds": float(config["loop_start_seconds"]),
        "loop_end_seconds": float(config["loop_end_seconds"]),
        "files": files,
    }
    output_directory.mkdir(parents=True, exist_ok=True)
    (output_directory / "generation_metadata.json").write_text(
        json.dumps(metadata, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    return metadata


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", type=Path, default=Path("config/audio_synthesis_v10.yaml"))
    parser.add_argument("--output", type=Path, default=Path("game/assets/audio/engines/v10_prototype"))
    args = parser.parse_args()
    metadata = generate(args.config, args.output)
    print(f"Generated {metadata['bank_name']} at {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

