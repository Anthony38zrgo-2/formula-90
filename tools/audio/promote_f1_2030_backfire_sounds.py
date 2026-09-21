#!/usr/bin/env python3
"""Promote the reviewed 500_backfire3..7 and limiter/TC samples into the bank.

These are additive backfire variants for the existing `engine_backfire` role
(the mixer cycles every bank sample carrying that role on a backfire trigger)
plus the `limiter_hit` / `tc_cut` one-shot accents fired on physical telemetry
edges. Processing mirrors `promote_vehicle_replacement_sounds.py` (mono PCM16,
44.1 kHz, DC removed, short edge fades, peak 0.89) and the manifest is updated
deterministically.
"""
from __future__ import annotations

import argparse
import hashlib
from collections.abc import Sequence
from pathlib import Path

import numpy as np

from tools.audio.bank_manifest import BankManifest, FileEntry, sha256_file
from tools.audio.dsp_common import (
    SAMPLE_RATE,
    dc_offset,
    lufs_approx,
    peak_abs,
    read_wav_mono,
    resample_mono,
    write_wav_mono16,
)

TARGET_PEAK = 0.89
FADE_IN_FRAMES = round(0.002 * SAMPLE_RATE)
FADE_OUT_FRAMES = round(0.012 * SAMPLE_RATE)
SOURCES = {f"backfire_{index}.wav": f"500_backfire{index}.wav" for index in range(3, 8)}
LIMITER_SOURCES = {"limiter_hit.wav": "limiter_hit.wav", "tc_cut.wav": "tc_cut.wav"}
ROLES = {"limiter_hit.wav": "limiter_hit", "tc_cut.wav": "tc_cut"}
PROVENANCE = "derived from original user-supplied replacement samples"
PROMOTION_RECIPE = "f1_2030_sfx_backfire_overlay_v1"
LIMITER_RECIPE = "f1_2030_sfx_limiter_tc_overlay_v1"


def _process(path: Path) -> tuple[np.ndarray, int]:
    source_rate, raw = read_wav_mono(path)
    samples = resample_mono(raw, source_rate, SAMPLE_RATE).astype(np.float64)
    samples = samples - float(np.mean(samples))
    fade_in = min(FADE_IN_FRAMES, len(samples))
    fade_out = min(FADE_OUT_FRAMES, len(samples))
    if fade_in:
        samples[:fade_in] *= np.arange(fade_in) / max(fade_in - 1, 1)
    if fade_out:
        samples[len(samples) - fade_out :] *= np.arange(fade_out - 1, -1, -1) / max(
            fade_out - 1, 1
        )
    peak = float(np.max(np.abs(samples)))
    if peak > 1e-12:
        samples = samples * (TARGET_PEAK / peak)
    samples = np.clip(samples, -0.999, 0.999)
    return samples, source_rate


def promote(source_dir: Path, bank_dir: Path, repo_root: Path) -> BankManifest:
    manifest_path = bank_dir / "bank_manifest.json"
    manifest = BankManifest.load(manifest_path)
    entries = {entry.file: entry for entry in manifest.files}

    def add(filename: str, source_name: str, role: str, recipe: str) -> None:
        source = source_dir / source_name
        if not source.is_file():
            raise FileNotFoundError(source)
        samples, source_rate = _process(source)
        output = bank_dir / filename
        write_wav_mono16(output, samples, SAMPLE_RATE)
        source_rel = source.resolve().relative_to(repo_root.resolve()).as_posix()
        entries[filename] = FileEntry(
            file=filename,
            role=role,
            loop=False,
            duration_s=round(len(samples) / SAMPLE_RATE, 6),
            loudness_dbfs=round(lufs_approx(samples.tolist()), 2),
            peak=round(peak_abs(samples.tolist()), 6),
            dc_offset=round(dc_offset(samples.tolist()), 6),
            synthesis={
                "category": "engine",
                "source_file": source_rel,
                "source_rate": source_rate,
                "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
                "promotion_recipe": recipe,
                "processing": {
                    "remove_dc": True,
                    "source_rate": source_rate,
                    "target_peak": TARGET_PEAK,
                    "target_rate": SAMPLE_RATE,
                    "fade_in_frames": FADE_IN_FRAMES,
                    "fade_out_frames": FADE_OUT_FRAMES,
                },
            },
            provenance=PROVENANCE,
            sha256=sha256_file(output),
        )

    for filename, source_name in SOURCES.items():
        add(filename, source_name, "engine_backfire", PROMOTION_RECIPE)
    for filename, source_name in LIMITER_SOURCES.items():
        add(filename, source_name, ROLES[filename], LIMITER_RECIPE)

    manifest.files = sorted(entries.values(), key=lambda entry: entry.file)
    manifest.generator = (
        "formula90s canonical bank + f1_2026_2008 replacement overlay v1 + "
        "f1_2030 backfire overlay v1 + f1_2030 limiter/tc overlay v1"
    )
    manifest.write(manifest_path)
    return manifest


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--bank", type=Path, required=True)
    parser.add_argument("--repo-root", type=Path, default=Path("."))
    args = parser.parse_args(argv)
    manifest = promote(args.source, args.bank, args.repo_root)
    print(
        f"Promoted {len(SOURCES)} backfire variants and "
        f"{len(LIMITER_SOURCES)} limiter/TC accents; bank now has {len(manifest.files)} entries."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
