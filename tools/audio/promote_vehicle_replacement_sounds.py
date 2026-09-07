"""Promote the reviewed active-vehicle replacement WAVs into the canonical bank.

The incoming files are preserved as provenance. Runtime WAVs are deterministically
converted to the canonical mono PCM16/44.1 kHz contract, DC-corrected, peak
normalized, and made seamless when they are continuous voices.
"""
from __future__ import annotations

import argparse
import hashlib
from collections.abc import Sequence
from pathlib import Path

from tools.audio.bank_manifest import BankManifest, FileEntry, sha256_file
from tools.audio.dsp_common import (
    SAMPLE_RATE,
    dc_offset,
    lufs_approx,
    make_loop_seamless,
    normalize_peak,
    peak_abs,
    read_wav_mono,
    remove_dc,
    resample_mono,
    write_wav_mono16,
)
from tools.audio.playback_metadata import playback_for_key

REPLACEMENTS: dict[str, tuple[str, bool, str]] = {
    "engine_high.wav": ("engine_high", True, "engine"),
    "engine_mid.wav": ("engine_mid", True, "engine"),
    "impact_hit_1.wav": ("impact_hit", False, "impact"),
    "impact_hit_2.wav": ("impact_hit", False, "impact"),
    "impact_hit_3.wav": ("impact_hit", False, "impact"),
    "impact_hit_4.wav": ("impact_hit", False, "impact"),
    "int_backfire.wav": ("engine_backfire", False, "engine"),
    "int_backfire_2.wav": ("engine_backfire", False, "engine"),
    "shift_up.wav": ("shift_up", False, "shift"),
    "shift_down.wav": ("shift_down", False, "shift"),
    "tyre_scrub.wav": ("tyre_scrub", True, "tire"),
}

LOOP_XFADE_FRAMES = 2048
TARGET_PEAK = 0.89
SHIFT_FADE_IN_FRAMES = round(0.002 * SAMPLE_RATE)
SHIFT_FADE_OUT_FRAMES = round(0.012 * SAMPLE_RATE)


def _source_hash(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _fade_edges(samples: list[float], fade_in: int, fade_out: int) -> list[float]:
    faded = samples.copy()
    for index in range(min(fade_in, len(faded))):
        faded[index] *= index / max(fade_in - 1, 1)
    for offset in range(min(fade_out, len(faded))):
        index = len(faded) - 1 - offset
        faded[index] *= offset / max(fade_out - 1, 1)
    return faded


def _process(path: Path, loop: bool) -> tuple[list[float], int, dict[str, object]]:
    source_rate, raw = read_wav_mono(path)
    samples = resample_mono(raw, source_rate, SAMPLE_RATE).tolist()
    processing: dict[str, object] = {
        "source_rate": source_rate,
        "target_rate": SAMPLE_RATE,
        "remove_dc": True,
        "target_peak": TARGET_PEAK,
    }
    if path.name == "tyre_scrub.wav":
        start = round(0.20 * SAMPLE_RATE)
        end = min(len(samples), round(2.50 * SAMPLE_RATE))
        samples = samples[start:end]
        processing["stable_loop_region_s"] = [0.20, 2.50]
    samples = remove_dc(samples)
    if loop:
        samples = make_loop_seamless(samples, LOOP_XFADE_FRAMES)
        processing["equal_power_loop_crossfade_frames"] = LOOP_XFADE_FRAMES
    # Loop folding can reintroduce a small mean; subtracting a constant preserves
    # the exact endpoint equality while restoring the bank DC contract.
    samples = remove_dc(samples)
    if path.name in {"shift_up.wav", "shift_down.wav"}:
        samples = _fade_edges(samples, SHIFT_FADE_IN_FRAMES, SHIFT_FADE_OUT_FRAMES)
        processing["fade_in_frames"] = SHIFT_FADE_IN_FRAMES
        processing["fade_out_frames"] = SHIFT_FADE_OUT_FRAMES
    samples = normalize_peak(samples, TARGET_PEAK)
    return samples, source_rate, processing


def promote(source_dir: Path, bank_dir: Path, repo_root: Path) -> BankManifest:
    manifest_path = bank_dir / "bank_manifest.json"
    manifest = BankManifest.load(manifest_path)
    entries = {entry.file: entry for entry in manifest.files}

    for filename, (role, loop, category) in REPLACEMENTS.items():
        source = source_dir / filename
        if not source.is_file():
            raise FileNotFoundError(source)
        samples, source_rate, processing = _process(source, loop)
        output = bank_dir / filename
        write_wav_mono16(output, samples, SAMPLE_RATE)
        source_rel = source.resolve().relative_to(repo_root.resolve()).as_posix()
        synthesis: dict[str, object] = {
            "category": category,
            "source_file": source_rel,
            "source_rate": source_rate,
            "source_sha256": _source_hash(source),
            "promotion_recipe": "f1_2026_2008_replacement_overlay_v1",
            "processing": processing,
        }
        entries[filename] = FileEntry(
            file=filename,
            role=role,
            loop=loop,
            duration_s=round(len(samples) / SAMPLE_RATE, 6),
            loop_start_s=0.0 if loop else None,
            loop_end_s=round(len(samples) / SAMPLE_RATE, 6) if loop else None,
            loudness_dbfs=round(lufs_approx(samples), 2),
            peak=round(peak_abs(samples), 6),
            dc_offset=round(dc_offset(samples), 6),
            playback=playback_for_key(filename.removesuffix(".wav")),
            synthesis=synthesis,
            provenance="derived from original user-supplied replacement samples",
            sha256=sha256_file(output),
        )

    manifest.files = sorted(entries.values(), key=lambda entry: entry.file)
    manifest.generator = "formula90s canonical bank + f1_2026_2008 replacement overlay v1"
    manifest.write(manifest_path)
    return manifest


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--source",
        type=Path,
        required=True,
        help="Reviewed replacement sources (explicit; no runtime default)",
    )
    parser.add_argument(
        "--bank",
        type=Path,
        required=True,
        help="Explicit bank destination; prefer promote_content.ps1 for runtime cutover",
    )
    parser.add_argument("--repo-root", type=Path, default=Path("."))
    args = parser.parse_args(argv)
    manifest = promote(args.source, args.bank, args.repo_root)
    print(f"Promoted {len(REPLACEMENTS)} sounds; bank now has {len(manifest.files)} entries.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
