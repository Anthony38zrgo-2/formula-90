#!/usr/bin/env python3
"""Offline deterministic scenario renderer — single responsibility: mix bank assets per scenario.

Consumes a deterministic `Scenario` (RPM, speed, throttle, gear, slip, surface,
impact events), crossfades bank loops at their wrap points, and emits an
auditionable WAV + CSV/JSON preview under scratch/audio/scenarios/. No GEVP/Godot coupling.

Bank contract: game/sounds/banks/v10_vehicle/*.wav + bank_manifest.json
Scenario definitions live in `scenarios.py`; DSP helpers in `dsp_common.py`.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import struct
import wave
from collections.abc import Sequence
from pathlib import Path

from tools.audio.bank_manifest import BankManifest
from tools.audio.dsp_common import SAMPLE_RATE, normalize_peak, write_wav_mono16
from tools.audio.scenarios import SCENARIO_FACTORIES, Scenario
from tools.common.output_policy import OutputMode, OutputPolicyError, validate_output_path

BANK_DIR = Path("game/sounds/banks/v10_vehicle")
DEFAULT_OUTPUT = Path("scratch/audio/scenarios")

# Loop-wrap crossfade (~11.6 ms) so bank loops never click when wrapped by the mixer.
LOOP_XFADE = 512

# Scenario surface token -> bank key (no asphalt bed: on-road is engine only).
SURFACE_KEY = {
    "asphalt": None,  # no tire bed
    "sand": "surf_sand",
    "grass": "surf_grass",
    "rumble": "surf_rumble",
}

# Scenario impact kind -> bank key.
IMPACT_KEY = {
    "cone": "impact_cone",
    "barrier": "impact_barrier",
    "hit_1": "impact_hit_1",
    "hit_2": "impact_hit_2",
    "hit_3": "impact_hit_3",
    "hit_4": "impact_hit_4",
    "fire": "impact_fire",
    "scrape": "impact_scrape",
}


def _load_bank_wavs(bank_dir: Path) -> dict[str, list[float]]:
    out: dict[str, list[float]] = {}
    for wav in sorted(bank_dir.glob("*.wav")):
        with wave.open(str(wav), "rb") as r:
            raw = r.readframes(r.getnframes())
        vals = [v[0] / 32767.0 for v in struct.iter_unpack("<h", raw[: len(raw) // 2 * 2])] if raw else []
        out[wav.stem] = vals
    return out


def _engine_band_for_rpm(rpm: float) -> tuple[str, str, float]:
    """Map RPM to engine_* bank keys (idle/low/mid/high/redline from the 98_int set)."""
    centers = [("engine_idle", 2200), ("engine_low", 4500), ("engine_mid", 7000), ("engine_high", 10000), ("engine_redline", 14500)]
    if rpm <= centers[0][1]:
        return centers[0][0], centers[0][0], 0.0
    if rpm >= centers[-1][1]:
        return centers[-1][0], centers[-1][0], 0.0
    for i in range(len(centers) - 1):
        lo, hi = centers[i], centers[i + 1]
        if lo[1] <= rpm <= hi[1]:
            u = (rpm - lo[1]) / (hi[1] - lo[1])
            return lo[0], hi[0], u
    return centers[2][0], centers[2][0], 0.0


def _make_loop_reader(buf: list[float]):
    """Return a callable step -> sample that crossfades at the wrap point."""
    n = len(buf)

    def read(step: int) -> float:
        if n == 0:
            return 0.0
        if n <= LOOP_XFADE:
            return buf[step % n]
        pos = step % n
        if pos >= n - LOOP_XFADE:
            k = pos - (n - LOOP_XFADE)
            t = k / max(1, LOOP_XFADE - 1)
            return buf[pos] * (1.0 - t) + buf[k] * t
        return buf[pos]

    return read


def render_scenario(scenario: Scenario, bank_dir: Path, out_wav: Path, out_csv: Path, out_json: Path) -> dict:
    bank_wavs = _load_bank_wavs(bank_dir)
    manifest = None
    mp = bank_dir / "bank_manifest.json"
    if mp.is_file():
        try:
            manifest = BankManifest.load(mp)
        except (OSError, ValueError, KeyError, TypeError):
            pass
    sr = SAMPLE_RATE
    n = int(scenario.duration_s * sr)
    mix: list[float] = [0.0] * n
    frames = scenario.frames
    if not frames:
        raise ValueError("scenario has no frames")

    def frame_at(t: float) -> object:
        idx = min(len(frames) - 1, max(0, int(t * 200)))
        while idx + 1 < len(frames) and frames[idx + 1].t <= t:
            idx += 1
        while idx > 0 and frames[idx].t > t:
            idx -= 1
        return frames[idx]

    readers = {name: _make_loop_reader(buf) for name, buf in bank_wavs.items()}
    impact_buffers = {k: bank_wavs.get(k, []) for k in IMPACT_KEY.values()}
    report_rows: list[dict] = []
    last_report_t = -1.0
    impact_events: list[tuple[int, list[float], float]] = []
    for imp in scenario.impacts:
        key = IMPACT_KEY.get(imp.kind, "impact_hit_1")
        buf = impact_buffers.get(key, [])
        if buf:
            impact_events.append((int(imp.t * sr), buf, imp.gain))

    for i in range(n):
        t = i / sr
        fr = frame_at(t)
        lo_band, hi_band, band_xfade = _engine_band_for_rpm(fr.rpm)
        e_lo = readers.get(lo_band, lambda _: 0.0)(i)
        e_hi = readers.get(hi_band, lambda _: 0.0)(i)
        engine = (1 - band_xfade) * e_lo + band_xfade * e_hi
        engine *= 0.45 + 0.55 * max(0.0, min(1.0, fr.throttle))
        # Surface bed (asphalt has none; sand/grass/rumble only).
        surf_key = SURFACE_KEY.get(fr.surface)
        tire = 0.0
        if surf_key is not None:
            surf_s = readers.get(surf_key, lambda _: 0.0)(i)
            tire_gain = 0.25 + 0.55 * max(0.0, min(1.0, fr.slip)) + 0.12 * min(1.0, fr.speed_kmh / 120.0)
            tire = surf_s * tire_gain
        s = engine * 0.62 + tire * 0.50
        for onset, buf, g in impact_events:
            j = i - onset
            if 0 <= j < len(buf):
                s += buf[j] * g * 0.85
        mix[i] = s
        if t - last_report_t >= 0.02 - 1e-9:
            last_report_t = t
            report_rows.append({"t": round(t, 4), "rpm": round(fr.rpm, 1), "speed_kmh": round(fr.speed_kmh, 1),
                                "throttle": round(fr.throttle, 3), "gear": fr.gear, "slip": round(fr.slip, 3),
                                "surface": fr.surface, "level_dbfs": round(20 * math.log10(max(1e-6, abs(s))), 2) if s != 0 else -90.0})

    mix = normalize_peak(mix, 0.89)
    write_wav_mono16(out_wav, mix, sr)
    out_csv.parent.mkdir(parents=True, exist_ok=True)
    with open(out_csv, "w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=["t", "rpm", "speed_kmh", "throttle", "gear", "slip", "surface", "level_dbfs"])
        w.writeheader()
        w.writerows(report_rows)
    peak = max(abs(v) for v in mix) if mix else 0.0
    rms_val = math.sqrt(sum(v * v for v in mix) / len(mix)) if mix else 0.0
    report = {"scenario": scenario.name, "description": scenario.description, "duration_s": scenario.duration_s,
              "sample_rate": sr, "channels": 1, "frames": len(report_rows), "peak": round(peak, 6),
              "rms": round(rms_val, 6), "events": [{"t": e.t, "kind": e.kind, "gain": e.gain} for e in scenario.impacts],
              "bank": manifest.bank_name if manifest else "unknown",
              "wav": str(out_wav).replace("\\", "/"), "csv": str(out_csv).replace("\\", "/")}
    out_json.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return report


def main(argv: Sequence[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="Render audio scenario previews offline")
    ap.add_argument("--repo-root", type=Path, default=Path("."))
    ap.add_argument("--bank", type=Path, default=BANK_DIR)
    ap.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    ap.add_argument("--scenario", type=str, default="all", help="name or 'all'")
    args = ap.parse_args(argv)
    try:
        root = args.repo_root.resolve(strict=True)
        output = validate_output_path(root, args.out, OutputMode.PREVIEW).path
    except (OSError, OutputPolicyError) as error:
        ap.exit(2, f"audio-scenario-preview: {error}\n")
    bank_dir: Path = args.bank if args.bank.is_absolute() else root / args.bank
    if not bank_dir.is_dir():
        print(f"Bank not found: {bank_dir}")
        return 2
    names = list(SCENARIO_FACTORIES) if args.scenario == "all" else [args.scenario]
    for name in names:
        fn = SCENARIO_FACTORIES.get(name)
        if not fn:
            print(f"Unknown scenario: {name} (choices: {', '.join(SCENARIO_FACTORIES)})")
            return 2
        sc = fn()
        out_wav = output / f"{sc.name}.wav"
        out_csv = output / f"{sc.name}.csv"
        out_json = output / f"{sc.name}.json"
        rep = render_scenario(sc, bank_dir, out_wav, out_csv, out_json)
        print(f"{sc.name}: peak {rep['peak']:.3f} rms {rep['rms']:.4f} -> {out_wav}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
