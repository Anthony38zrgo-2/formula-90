"""Summarize the V10-001..006 listening capture and physical telemetry."""

from __future__ import annotations

import csv
import json
import math
import statistics
import sys
import wave
from array import array
from pathlib import Path


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit("usage: analyze_v10_001_006.py <wav> <physical.csv> <output.json>")
    wav_path, csv_path, output_path = map(Path, sys.argv[1:])

    rows = []
    with csv_path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            rows.append(
                {
                    "time_s": float(row["time_s"]),
                    "rpm": float(row["rpm"]),
                    "pressure_pa": float(row["chamber_pressure_pa"]),
                    "temperature_k": float(row["chamber_temperature_k"]),
                    "heat_release": float(row["heat_release_rate"]),
                    "phase": int(row["phase"]),
                }
            )

    phases = {}
    for phase in range(4):
        selected = [row for row in rows if row["phase"] == phase]
        phases[str(phase)] = {
            "samples": len(selected),
            "pressure_pa_median": statistics.median(row["pressure_pa"] for row in selected),
            "pressure_pa_min": min(row["pressure_pa"] for row in selected),
            "pressure_pa_max": max(row["pressure_pa"] for row in selected),
            "temperature_k_median": statistics.median(row["temperature_k"] for row in selected),
            "heat_release_max": max(row["heat_release"] for row in selected),
        }

    with wave.open(str(wav_path), "rb") as audio:
        frames = audio.readframes(audio.getnframes())
        sample_width = audio.getsampwidth()
        if audio.getnchannels() != 1 or sample_width != 2:
            raise SystemExit("expected mono PCM16 WAV")
        samples = array("h")
        samples.frombytes(frames)
        if sys.byteorder != "little":
            samples.byteswap()
        scale = float(2 ** (8 * sample_width - 1))
        normalized = [sample / scale for sample in samples]
        audio_summary = {
            "sample_rate_hz": audio.getframerate(),
            "frames": audio.getnframes(),
            "duration_s": audio.getnframes() / audio.getframerate(),
            "peak": max(abs(sample) for sample in normalized),
            "rms": math.sqrt(sum(sample * sample for sample in normalized) / len(normalized)),
        }

    first = rows[0]
    lift_start = next(row for row in rows if row["time_s"] >= 5.0)
    last = rows[-1]
    output = {
        "schema": "v10-001-006-sweep-analysis-v1",
        "wav": str(wav_path),
        "physical_telemetry": str(csv_path),
        "telemetry_rows": len(rows),
        "finite_bounded": all(
            math.isfinite(row[key])
            for row in rows
            for key in ("rpm", "pressure_pa", "temperature_k", "heat_release")
        ),
        "phase_counts": {phase: data["samples"] for phase, data in phases.items()},
        "phase_summary": phases,
        "trajectory": {
            "first_rpm": first["rpm"],
            "lift_start_rpm": lift_start["rpm"],
            "last_rpm": last["rpm"],
            "lift_start_time_s": lift_start["time_s"],
            "last_time_s": last["time_s"],
        },
        "audio": audio_summary,
        "metadata": json.loads(wav_path.with_suffix(".metadata.json").read_text(encoding="utf-8-sig")),
    }
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(output, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(output, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
