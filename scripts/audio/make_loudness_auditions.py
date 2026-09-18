"""Build loudness-matched audition copies (BS.1770) for A/B evidence.

Every input is normalized to the quietest input's integrated loudness so no
copy is boosted above its rendered level, then hard-capped at the requested
peak ceiling. A CSV with applied gains and final metrics is written next to
the auditions for traceability.
"""

from __future__ import annotations

import argparse
import csv
import glob
from pathlib import Path

import numpy as np
import pyloudnorm as pyln
import soundfile as sf


def collect_inputs(inputs: list[str]) -> list[Path]:
    files: list[Path] = []
    for raw in inputs:
        path = Path(raw)
        if path.is_dir():
            files.extend(sorted(path.rglob("*.wav")))
        elif path.is_file():
            files.append(path)
        else:
            matches = sorted(Path(match) for match in glob.glob(raw, recursive=True))
            if not matches:
                raise SystemExit(f"input not found: {raw}")
            files.extend(matches)
    return files


def output_name(path: Path) -> str:
    return f"{path.parent.name}_{path.stem}.wav"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inputs", nargs="+", help="WAV files or globs")
    parser.add_argument("--out-dir", type=Path, required=True)
    parser.add_argument("--peak-ceiling", type=float, default=0.999)
    parser.add_argument("--csv", type=Path, default=None)
    args = parser.parse_args()

    files = collect_inputs(args.inputs)
    if not files:
        raise SystemExit("no WAV files found")

    signals: list[tuple[Path, np.ndarray, int, float]] = []
    for path in files:
        data, sample_rate = sf.read(path, dtype="float64", always_2d=True)
        mono = data.mean(axis=1)
        loudness = float(pyln.Meter(sample_rate).integrated_loudness(mono))
        signals.append((path, data, sample_rate, loudness))

    target = min(loudness for _, _, _, loudness in signals)
    args.out_dir.mkdir(parents=True, exist_ok=True)
    rows: list[list[object]] = []
    for path, data, sample_rate, loudness in signals:
        gain = 10.0 ** ((target - loudness) / 20.0)
        scaled = data * gain
        peak = float(np.max(np.abs(scaled))) if scaled.size else 0.0
        if peak > args.peak_ceiling:
            scaled *= args.peak_ceiling / peak
            peak = args.peak_ceiling
        destination = args.out_dir / output_name(path)
        sf.write(destination, scaled, sample_rate, subtype="PCM_16")
        rows.append([str(path), str(destination), loudness, target, gain, peak])
        print(
            f"{path.parent.name}/{path.name:16s} lufs={loudness:6.2f} gain={20 * np.log10(gain):+5.2f} dB "
            f"peak={20 * np.log10(max(peak, 1e-12)):6.2f} dBFS"
        )
    print(f"\ntarget (min LUFS): {target:.2f} -> {args.out_dir}")
    csv_path = args.csv or (args.out_dir / "audition_gains.csv")
    with csv_path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(["source", "audition", "lufs_in", "target_lufs", "gain_linear", "peak_out"])
        writer.writerows(rows)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
