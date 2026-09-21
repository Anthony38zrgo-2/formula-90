#!/usr/bin/env python3
"""Technical validation of the Grand Prix sampler listening package.

Checks the rendered WAVs against the focused automated gates: finite output,
no hard clipping, stem algebra before the nonlinear master, steady-hold level
continuity, sweep continuity within 2 dB of the local trend, ascending versus
descending consistency at equal RPM, event audition counts and runtime
diagnostics. Writes reports/audio-v10/grand-prix-sampler/validation.json.
"""
from __future__ import annotations

import json
import struct
import sys
import wave
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
RENDERS = ROOT / "reports/audio-v10/grand-prix-sampler/renders"
REPORT = ROOT / "reports/audio-v10/grand-prix-sampler/validation.json"

SAMPLE_RATE = 44100
HOLD_FRAMES = 26460
SWEEP_TICKS = 800
SWEEP_TICK_FRAMES = 441


def read_wav(path: Path) -> np.ndarray:
    with wave.open(str(path), "rb") as handle:
        channels = handle.getnchannels()
        width = handle.getsampwidth()
        rate = handle.getframerate()
        frames = handle.getnframes()
        raw = handle.readframes(frames)
    dtype = {1: np.int8, 2: np.int16, 4: np.int32}[width]
    data = np.frombuffer(raw, dtype=dtype).astype(np.float64) / float(1 << (8 * width - 1))
    if channels == 2:
        data = data.reshape(-1, 2)
    else:
        data = data.reshape(-1, 1)
    assert rate == SAMPLE_RATE
    return data


def rms(samples: np.ndarray) -> float:
    return float(np.sqrt(np.mean(samples.astype(np.float64) ** 2))) if samples.size else 0.0


def dbfs(value: float) -> float:
    return 20.0 * np.log10(max(value, 1e-12))


def envelope_rms(samples: np.ndarray, window: int, hop: int) -> np.ndarray:
    mono = samples.mean(axis=1) if samples.ndim > 1 else samples
    count = max(1, (len(mono) - window) // hop + 1)
    return np.array(
        [rms(mono[index * hop: index * hop + window]) for index in range(count)]
    )


def smooth(values: np.ndarray, size: int = 9) -> np.ndarray:
    if len(values) < size:
        return values
    kernel = np.ones(size) / size
    padded = np.pad(values, (size // 2, size // 2), mode="edge")
    return np.convolve(padded, kernel, mode="valid")


def burst_count(samples: np.ndarray, threshold_ratio: float = 0.08) -> int:
    mono = np.abs(samples.mean(axis=1))
    envelope = envelope_rms(samples, 2205, 441)
    if envelope.size == 0:
        return 0
    threshold = envelope.max() * threshold_ratio
    active = envelope > threshold
    count = 0
    previous = False
    for value in active:
        if value and not previous:
            count += 1
        previous = bool(value)
    return count


def main() -> int:
    manifest = json.loads((RENDERS / "render_manifest.json").read_text(encoding="utf-8"))
    checks: list[dict] = []
    failures: list[str] = []

    def check(name: str, passed: bool, detail: str) -> None:
        checks.append({"name": name, "passed": bool(passed), "detail": detail})
        if not passed:
            failures.append(f"{name}: {detail}")

    files = {entry["file"]: entry for entry in manifest["files"]}

    peaks = {name: entry["peak"] for name, entry in files.items()}
    worst_peak = max(peaks.values())
    check(
        "output_peak_below_full_scale",
        worst_peak <= 0.999,
        f"worst rendered peak {worst_peak:.4f}",
    )

    full_sampler = read_wav(RENDERS / "full_grand_prix_sampler.wav")
    full_reference = read_wav(RENDERS / "full_gf509_reference.wav")
    sampler_rms = rms(full_sampler)
    reference_rms = rms(full_reference)
    check(
        "both_backends_render_audio",
        sampler_rms > 1e-4 and reference_rms > 1e-4,
        f"sampler {dbfs(sampler_rms):.2f} dBFS, reference {dbfs(reference_rms):.2f} dBFS",
    )
    check(
        "rendered_samples_finite",
        bool(np.isfinite(full_sampler).all() and np.isfinite(full_reference).all()),
        "all full-mix samples finite",
    )

    stem_names = {
        "engine": "stem_engine.wav",
        "gearbox": "stem_gearbox.wav",
        "backfire": "stem_backfire.wav",
        "limiter": "stem_limiter.wav",
        "preserved": "stem_preserved_effects.wav",
    }
    stems = {key: read_wav(RENDERS / name) for key, name in stem_names.items()}
    pre_master = read_wav(RENDERS / "stem_pre_master_sum.wav")
    stem_sum = sum(stems.values())
    difference = np.abs(stem_sum - pre_master)
    check(
        "stems_reconstruct_pre_master_sum",
        float(difference.max()) < 1e-4,
        f"max stem-sum error {float(difference.max()):.3e} (16-bit quantization)",
    )
    check(
        "stems_are_not_the_post_master_mix",
        float(np.abs(pre_master - full_sampler).max()) > 1e-4,
        "shared nonlinear master alters the sum as expected",
    )
    stem_energy = {key: rms(value) for key, value in stems.items()}
    check(
        "engine_and_event_stems_non_silent",
        stem_energy["engine"] > 1e-4
        and stem_energy["gearbox"] > 1e-6
        and stem_energy["backfire"] > 1e-6
        and stem_energy["limiter"] > 1e-6,
        ", ".join(f"{key}={dbfs(value):.1f}dB" for key, value in stem_energy.items()),
    )

    holds = read_wav(RENDERS / "steady_holds.wav")
    hold_count = len(holds) // HOLD_FRAMES
    hold_levels = [
        dbfs(rms(holds[index * HOLD_FRAMES + HOLD_FRAMES // 2: (index + 1) * HOLD_FRAMES]))
        for index in range(hold_count)
    ]
    steps = [
        abs(hold_levels[index + 1] - hold_levels[index])
        for index in range(len(hold_levels) - 1)
    ]
    check(
        "steady_hold_levels_continuous",
        max(steps) <= 4.0,
        "hold levels dB: " + ", ".join(f"{level:.1f}" for level in hold_levels),
    )
    check(
        "steady_hold_levels_bounded",
        (max(hold_levels) - min(hold_levels)) <= 12.0,
        f"level span {max(hold_levels) - min(hold_levels):.1f} dB",
    )

    for direction, filename in (("ascending", "sweep_ascending.wav"), ("descending", "sweep_descending.wav")):
        sweep = read_wav(RENDERS / filename)
        envelope = envelope_rms(sweep, 2205, 441)
        trend = smooth(envelope, 9)
        with np.errstate(divide="ignore"):
            residual_db = 20.0 * np.log10((envelope + 1e-12) / (trend + 1e-12))
        worst = float(np.max(np.abs(residual_db)))
        check(
            f"sweep_{direction}_level_continuity",
            worst <= 2.0,
            f"max |residual| {worst:.2f} dB over {len(envelope)} windows",
        )

    ascending = envelope_rms(read_wav(RENDERS / "sweep_ascending.wav"), 2205, 441)
    descending = envelope_rms(read_wav(RENDERS / "sweep_descending.wav"), 2205, 441)
    length = min(len(ascending), len(descending))
    if length > 0:
        ascending = ascending[:length]
        descending = descending[:length][::-1]
        with np.errstate(divide="ignore"):
            difference_db = np.abs(20.0 * np.log10((ascending + 1e-12) / (descending + 1e-12)))
        check(
            "sweep_directions_match_at_equal_rpm",
            float(np.median(difference_db)) <= 2.0,
            f"median direction difference {float(np.median(difference_db)):.2f} dB",
        )

    downshift_stem = read_wav(RENDERS / "audition_downshift_gearbox_stem.wav")
    backfire_stem = read_wav(RENDERS / "audition_backfire_stem.wav")
    check(
        "downshift_variants_audible",
        burst_count(downshift_stem, threshold_ratio=0.15) >= 3,
        f"bursts detected: {burst_count(downshift_stem, threshold_ratio=0.15)}",
    )
    check(
        "backfire_variants_audible",
        burst_count(backfire_stem, threshold_ratio=0.15) >= 3,
        f"bursts detected: {burst_count(backfire_stem, threshold_ratio=0.15)}",
    )

    diagnostics = manifest.get("sampler_diagnostics") or {}
    check(
        "no_event_overflow_or_late_events",
        diagnostics.get("overflowed_events", 0) == 0
        and diagnostics.get("late_events", 0) == 0,
        json.dumps(diagnostics),
    )
    check(
        "sequence_event_counts_match_script",
        diagnostics.get("accepted_upshift_events", 0) == 5
        and diagnostics.get("accepted_downshift_events", 0) == 3
        and diagnostics.get("accepted_backfire_events", 0) == 1
        and diagnostics.get("accepted_limiter_events", 0) == 1,
        f"upshifts={diagnostics.get('accepted_upshift_events')} "
        f"downshifts={diagnostics.get('accepted_downshift_events')} "
        f"backfires={diagnostics.get('accepted_backfire_events')} "
        f"limiter={diagnostics.get('accepted_limiter_events')}",
    )
    check(
        "backend_identity_recorded",
        bool(manifest.get("grand_prix_bank_sha256"))
        and manifest.get("sampler_backend_active") is True,
        f"bank sha {manifest.get('grand_prix_bank_sha256', '')[:16]}...",
    )

    report = {
        "schema_version": 1,
        "checks": checks,
        "failures": failures,
        "hold_levels_dbfs": hold_levels,
        "stem_rms_dbfs": {key: dbfs(value) for key, value in stem_energy.items()},
        "sampler_rms_dbfs": dbfs(sampler_rms),
        "reference_rms_dbfs": dbfs(reference_rms),
        "sampler_diagnostics": diagnostics,
    }
    REPORT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    for entry in checks:
        status = "ok  " if entry["passed"] else "FAIL"
        print(f"[{status}] {entry['name']}: {entry['detail']}")
    print(f"validation report: {REPORT}")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
