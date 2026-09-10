#!/usr/bin/env python3
"""Assemble the F2002 experimental bank (schema 2) from prepared loops.

Reads reports/audio-v10/f2002-experimental/prepared/*.sample-layer.json,
copies loop tonal/residual WAVs + per-source metadata into
game/audio/v10_f2002_experimental, and writes manifest.json (schema 2)
with roles, variant groups and SHA-256 of every runtime asset.
"""
from __future__ import annotations

import hashlib
import json
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PREPARED = ROOT / "reports/audio-v10/f2002-experimental/prepared"
BANK = ROOT / "game/audio/v10_f2002_experimental"
CALIB = ROOT / "reports/audio-v10/f2002-experimental/calibration/anchors.json"

ON_SOURCES = [
    "low_on",
    "med_on",
    "med_hi_on",
    "almost_hi_on",
    "hi_max_on",
    "hi_on",
    "max_on",
]
OFF_SOURCES = [
    "low_off",
    "med_off_front_near2",
    "hi_off",
    "hi_off_2",
    "max_off",
]
# Variant groups: members interpolate (equal power) over the group span.
GROUPS = {
    "on_zone_high": ["hi_max_on", "hi_on"],
    "off_zone_midhigh": ["hi_off", "hi_off_2"],
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    calib = json.loads(CALIB.read_text(encoding="utf-8"))
    anchors: dict = calib["anchors"]
    if BANK.exists():
        shutil.rmtree(BANK)
    BANK.mkdir(parents=True)
    member_of = {}
    for group, members in GROUPS.items():
        for position, member in enumerate(members):
            member_of[member] = {"group": group, "position": position, "count": len(members)}

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
            "rpm_source": "spectral_estimate_reviewed",
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

    manifest = {
        "schema_version": 2,
        "key": "v10_f2002_experimental",
        "output_gain": 0.61,
        "samples": [entry(s, "on") for s in ON_SOURCES],
        "off_samples": [entry(s, "off") for s in OFF_SOURCES],
    }
    (BANK / "manifest.json").write_text(
        json.dumps(manifest, indent=2), encoding="utf-8"
    )
    print(f"bank={BANK} on={len(manifest['samples'])} off={len(manifest['off_samples'])}")
    for group, members in GROUPS.items():
        print(f"group {group}: {members}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
