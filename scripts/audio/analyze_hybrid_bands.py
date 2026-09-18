"""Per-band evidence for V10 hybrid renders.

Reads WAV files (or directories) and reports RMS/peak, integrated loudness
(BS.1770 via pyloudnorm when available) and the energy share of each band
relative to the full-band power. Intended for A/B evidence, not as an audio
quality verdict.
"""

from __future__ import annotations

import argparse
import csv
import glob
import json
import math
import sys
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import welch

DEFAULT_BANDS = [(40, 150), (150, 350), (350, 800), (800, 2000), (2000, 5000), (5000, 10000)]


def parse_bands(spec: str) -> list[tuple[float, float]]:
    bands: list[tuple[float, float]] = []
    for chunk in spec.split(","):
        low, high = chunk.split("-")
        bands.append((float(low), float(high)))
    if not bands:
        raise argparse.ArgumentTypeError("empty band specification")
    return bands


def band_label(low: float, high: float) -> str:
    def human(value: float) -> str:
        return f"{value / 1000:g}k" if value >= 1000 else f"{value:g}"

    return f"{human(low)}-{human(high)}"


def load_mono(path: Path) -> tuple[np.ndarray, int]:
    data, sample_rate = sf.read(path, dtype="float64", always_2d=True)
    return data.mean(axis=1), sample_rate


def integrated_lufs(signal: np.ndarray, sample_rate: int) -> float | None:
    try:
        import pyloudnorm as pyln
    except ImportError:
        return None
    meter = pyln.Meter(sample_rate)
    return float(meter.integrated_loudness(signal))


def analyze(path: Path, bands: list[tuple[float, float]]) -> dict:
    signal, sample_rate = load_mono(path)
    rms = float(np.sqrt(np.mean(signal**2)))
    peak = float(np.max(np.abs(signal))) if signal.size else 0.0
    frequencies, power = welch(signal, fs=sample_rate, nperseg=8192)
    total = float(power.sum())
    band_power: dict[str, float] = {}
    for low, high in bands:
        mask = (frequencies >= low) & (frequencies < high)
        band_power[band_label(low, high)] = float(power[mask].sum())
    shares = {
        label: (10.0 * math.log10(value / total) if value > 0.0 and total > 0.0 else -300.0)
        for label, value in band_power.items()
    }
    levels = {
        label: (10.0 * math.log10(value) if value > 0.0 else -300.0)
        for label, value in band_power.items()
    }
    return {
        "path": str(path),
        "sample_rate": sample_rate,
        "frames": int(signal.size),
        "rms_dbfs": 20.0 * math.log10(rms) if rms > 0.0 else -300.0,
        "peak_dbfs": 20.0 * math.log10(peak) if peak > 0.0 else -300.0,
        "lufs": integrated_lufs(signal, sample_rate),
        "band_level_db": levels,
        "band_share_db": shares,
    }


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


def print_table(results: list[dict], bands: list[tuple[float, float]]) -> None:
    labels = [band_label(low, high) for low, high in bands]
    header = f"{'file':52s} {'RMS':>7s} {'peak':>7s} {'LUFS':>7s}  " + " ".join(
        f"{label:>9s}" for label in labels
    )
    print(header)
    print("-" * len(header))
    for result in results:
        lufs = "n/a" if result["lufs"] is None else f"{result['lufs']:.1f}"
        shares = " ".join(f"{result['band_share_db'][label]:9.1f}" for label in labels)
        print(
            f"{result['path'][-52:]:52s} {result['rms_dbfs']:7.1f} "
            f"{result['peak_dbfs']:7.1f} {lufs:>7s}  {shares}"
        )
    print("\nband values are power share in dB relative to full-band power")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inputs", nargs="+", help="WAV files or directories")
    parser.add_argument("--bands", type=parse_bands, default=DEFAULT_BANDS)
    parser.add_argument("--csv", type=Path, help="optional CSV output path")
    parser.add_argument("--json", type=Path, help="optional JSON output path")
    args = parser.parse_args()

    files = collect_inputs(args.inputs)
    if not files:
        print("no WAV files found", file=sys.stderr)
        return 1
    results = [analyze(path, args.bands) for path in files]
    print_table(results, args.bands)
    if args.csv:
        labels = [band_label(low, high) for low, high in args.bands]
        args.csv.parent.mkdir(parents=True, exist_ok=True)
        with args.csv.open("w", newline="", encoding="utf-8") as handle:
            writer = csv.writer(handle)
            writer.writerow(
                ["file", "rms_dbfs", "peak_dbfs", "lufs"]
                + [f"share_{label}" for label in labels]
                + [f"level_{label}" for label in labels]
            )
            for result in results:
                writer.writerow(
                    [result["path"], result["rms_dbfs"], result["peak_dbfs"], result["lufs"]]
                    + [result["band_share_db"][label] for label in labels]
                    + [result["band_level_db"][label] for label in labels]
                )
    if args.json:
        args.json.parent.mkdir(parents=True, exist_ok=True)
        args.json.write_text(json.dumps(results, indent=2), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
