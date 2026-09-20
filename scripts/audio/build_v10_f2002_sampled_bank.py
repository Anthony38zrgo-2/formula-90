#!/usr/bin/env python3
"""Assemble the fully-sampled V10 bank (schema 2) from prepared loops.

Reads reports/audio-v10/f2002-sampled/prepared/*.sample-layer.json and the
frozen anchors at reports/audio-v10/f2002-sampled/calibration/anchors.json,
copies loop tonal/residual WAVs + per-source metadata into
game/audio/v10_f2002_sampled, and writes manifest.json (schema 2) with roles,
variant groups and SHA-256 of every runtime asset.

This bank is the source for the sample-only engine variant: every source keeps
its measured native RPM; nothing is re-laddered. The gear-up/down one-shots
are copied from the experimental bank on first build.
"""
from __future__ import annotations

import hashlib
import json
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PREPARED = ROOT / "reports/audio-v10/f2002-sampled/prepared"
BANK = ROOT / "game/audio/v10_f2002_sampled"
CALIB = ROOT / "reports/audio-v10/f2002-sampled/calibration/anchors.json"
GEAR_SOURCE = ROOT / "game/audio/v10_f2002_experimental"

# Semantic classification (ON vs OFF) of the curated set that ships in this
# bank. `anchors.json:excluded` records the exterior takes (spectral audit) and
# the sources removed from implementation/sfx by user curation.
ON_STEMS = {
    "low_on",
    "med_on",
    "med_hi_on",
    "almost_hi_on",
    "hi_max_on",
    "hi_on",
    "max_on",
}
OFF_STEMS = {
    "low_off",
    "hi_off",
    "hi_off_2",
    "max_off",
}
# Audited on 2026-09-19 with the firing-share + loop-seam method.
AUDITED_STEMS = {
    "low_rear",
    "med_rear3",
    "med_rear_far2",
    "hi_rear_near",
    "max_rear_near",
    "max_front",
    "low_off_rear",
    "med_off_rear",
    "hi_off_rear",
}
STATIC_ASSETS = ["gearup.wav", "geardn.wav"]
GENERATED_SUFFIXES = (".sample-layer.json", ".loop.tonal.wav", ".loop.residual.wav")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def rpm_source(stem: str) -> str:
    if stem == "idle":
        return "explicit_reviewed"
    if stem in AUDITED_STEMS:
        return "audited_firing_share_loop_seam"
    return "spectral_estimate_reviewed"


def clean_generated(bank: Path) -> None:
    if not bank.exists():
        bank.mkdir(parents=True)
        return
    for path in bank.iterdir():
        if path.is_file() and (
            path.name == "manifest.json" or path.name.endswith(GENERATED_SUFFIXES)
        ):
            path.unlink()


def remove_excluded_artifacts(excluded_stems: set[str]) -> None:
    """Delete generated artifacts (and their Godot imports) for sources that are
    no longer part of the bank, so no stale zone assets linger on disk."""
    if not BANK.exists():
        return
    for stem in excluded_stems:
        for name in (
            f"{stem}.sample-layer.json",
            f"{stem}.loop.tonal.wav",
            f"{stem}.loop.residual.wav",
            f"{stem}.loop.tonal.wav.import",
            f"{stem}.loop.residual.wav.import",
        ):
            path = BANK / name
            if path.is_file():
                path.unlink()


def ensure_static_assets() -> int:
    for name in STATIC_ASSETS:
        target = BANK / name
        if target.is_file():
            continue
        source = GEAR_SOURCE / name
        if not source.is_file():
            print(f"missing static asset source: {source}", file=sys.stderr)
            return 1
        shutil.copy2(source, target)
    return 0


def main() -> int:
    calib = json.loads(CALIB.read_text(encoding="utf-8"))
    anchors: dict[str, float] = calib["anchors"]
    groups: dict[str, list[str]] = calib["groups"]
    excluded_stems = {
        name.removesuffix(".wav") for name in calib.get("excluded", {})
    }
    all_stems = {name.removesuffix(".wav") for name in anchors}
    kept = ON_STEMS | OFF_STEMS
    if all_stems - excluded_stems != kept or excluded_stems & kept:
        print(
            "anchor set does not match the interior/exterior classification: "
            f"missing={sorted(kept - (all_stems - excluded_stems))} "
            f"extra={sorted((all_stems - excluded_stems) - kept)} "
            f"overlap={sorted(excluded_stems & kept)}",
            file=sys.stderr,
        )
        return 1
    clean_generated(BANK)
    remove_excluded_artifacts(excluded_stems)
    BANK.mkdir(parents=True, exist_ok=True)
    if ensure_static_assets() != 0:
        return 1

    member_of: dict[str, dict] = {}
    for group, members in groups.items():
        for position, wav in enumerate(members):
            member_of[wav.removesuffix(".wav")] = {
                "group": group,
                "position": position,
                "count": len(members),
            }

    def entry(stem: str, role: str) -> dict:
        meta_name = f"{stem}.sample-layer.json"
        meta = json.loads((PREPARED / meta_name).read_text(encoding="utf-8"))
        tonal_name = meta["files"]["loop_tonal"]
        residual_name = meta["files"]["loop_residual"]
        shutil.copy2(PREPARED / tonal_name, BANK / tonal_name)
        shutil.copy2(PREPARED / residual_name, BANK / residual_name)
        shutil.copy2(PREPARED / meta_name, BANK / meta_name)
        record = {
            "key": stem,
            "role": role,
            "metadata": meta_name,
            "native_rpm": anchors[f"{stem}.wav"],
            "rpm_source": rpm_source(stem),
            "effective_loop_rpm": meta["loop_effective_rpm"],
            "sample_rate": meta["sample_rate"],
            "channels": 1,
            "loop_frames": meta["loop_length_samples"],
            "loop_engine_cycles_720": meta["loop_engine_cycles_720"],
            "firing_phase_degrees_at_loop_start": meta[
                "firing_phase_degrees_at_loop_start"
            ],
            "source_sha256": meta["source_sha256"],
            "tonal": tonal_name,
            "tonal_sha256": sha256(BANK / tonal_name),
            "residual": residual_name,
            "residual_sha256": sha256(BANK / residual_name),
        }
        if stem in member_of:
            record["variant"] = member_of[stem]
        return record

    on_order = sorted(ON_STEMS, key=lambda stem: anchors[f"{stem}.wav"])
    off_order = sorted(OFF_STEMS, key=lambda stem: anchors[f"{stem}.wav"])
    # Variant members must remain contiguous in anchor order; verify.
    for group, members in groups.items():
        member_stems = [wav.removesuffix(".wav") for wav in members]
        order = on_order if member_stems[0] in ON_STEMS else off_order
        positions = sorted(order.index(stem) for stem in member_stems)
        if positions != list(range(positions[0], positions[0] + len(positions))):
            print(f"group {group} is not contiguous in anchor order", file=sys.stderr)
            return 1

    manifest = {
        "schema_version": 2,
        "key": "v10_f2002_sampled",
        "output_gain": 0.61,
        "samples": [entry(stem, "on") for stem in on_order],
        "off_samples": [entry(stem, "off") for stem in off_order],
    }
    (BANK / "manifest.json").write_text(
        json.dumps(manifest, indent=2), encoding="utf-8"
    )
    print(f"bank={BANK} on={len(manifest['samples'])} off={len(manifest['off_samples'])}")
    for group, members in groups.items():
        print(f"group {group}: {members}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
