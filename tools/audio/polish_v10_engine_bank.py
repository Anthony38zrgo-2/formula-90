"""Apply the approved mastering polish to the canonical V10 engine loops.

The nine engine loops approved in the sonic audit are rewritten in place with
the exact chains frozen in ``APPROVED_POLISH_CHAINS``: the direct-current
offset is removed, each musical band hump and ringing resonance is attenuated
with the approved bells, and the result is root-mean-square level matched to
the untouched loop so runtime calibration stays valid. The provenance records
are updated in the same run: ``manifest.json`` waveform hashes and levels,
``source_inventory.json`` runtime hashes plus an ``engine_polish`` ledger
recording the applied settings and pre/post digests.

Compression was deliberately not applied: the audit measured low crest
factors for every loop. Events, the transmission whine loop and all metadata
goldens stay untouched. The tool refuses to rewrite twice: a second run can
only verify, through ``--check``, that the bank still matches the ledger.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path

import numpy as np
from tools.audio import build_canonical_v10_engine_bank as canonical
from tools.audio import evaluate_v10_engine_bank as evaluator
from tools.common.output_policy import validate_output_path

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_BANK_DIRECTORY = ROOT / "game/sounds/banks/v10-v2-bank"
TOOL_NAME = "tools/audio/polish_v10_engine_bank.py"
TOOL_REVISION = 1
MAXIMUM_POLISHED_PEAK = 0.95


def _bell(center_hz: float, gain_db: float, width_octaves: float, purpose: str) -> dict:
    return {
        "kind": "bell",
        "purpose": purpose,
        "center_hz": center_hz,
        "gain_db": gain_db,
        "width_octaves": width_octaves,
    }


def _musical(center_hz: float, gain_db: float, width_octaves: float) -> dict:
    return _bell(center_hz, gain_db, width_octaves, "musical_band_hump")


def _ringing(center_hz: float, gain_db: float, width_octaves: float) -> dict:
    return _bell(center_hz, gain_db, width_octaves, "ringing_notch")


APPROVED_POLISH_CHAINS: dict[str, list[dict]] = {
    "engine_low_off_loop": [
        _musical(500.0, -2.8, 0.746),
        _musical(200.0, -4.31, 0.9),
        _ringing(541.0, -5.85, 0.08),
    ],
    "engine_mid_off_loop": [
        _musical(1000.0, -2.42, 0.646),
        _musical(315.0, -2.74, 0.73),
        _musical(200.0, -4.19, 0.9),
        _musical(500.0, -4.5, 0.9),
        _ringing(522.2, -8.0, 0.08),
    ],
    "engine_off_maximum_loop": [
        _musical(630.0, -2.67, 0.712),
        _musical(250.0, -4.5, 0.9),
        _musical(400.0, -4.5, 0.9),
    ],
    "engine_idle_loop": [
        _musical(630.0, -1.96, 0.523),
        _musical(200.0, -4.5, 0.9),
        _musical(1000.0, -4.5, 0.9),
        _ringing(586.8, -6.49, 0.08),
    ],
    "engine_mid_on_loop": [
        _musical(200.0, -2.93, 0.781),
        _musical(1000.0, -3.83, 0.9),
        _musical(315.0, -4.1, 0.9),
        _musical(500.0, -4.5, 0.9),
    ],
    "engine_powered_high_loop": [
        _musical(2000.0, -2.23, 0.595),
        _musical(630.0, -4.5, 0.9),
        _musical(400.0, -4.5, 0.9),
    ],
    "engine_powered_low_loop": [
        _musical(1250.0, -3.09, 0.823),
        _musical(160.0, -4.3, 0.9),
        _musical(250.0, -4.5, 0.9),
        _musical(400.0, -4.5, 0.9),
        _ringing(371.4, -6.36, 0.08),
        _ringing(577.4, -7.97, 0.08),
    ],
    "engine_powered_maximum_loop": [
        _musical(1600.0, -1.86, 0.497),
        _musical(447.2, -2.22, 0.434),
        _musical(800.0, -2.92, 0.778),
    ],
    "engine_powered_medium_high_loop": [
        _musical(1118.0, -1.46, 0.434),
        _musical(500.0, -4.5, 0.9),
    ],
}


def sha256_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def decibels_full_scale(value: float) -> float:
    return 20.0 * math.log10(max(value, 1e-9))


def polish_samples(samples: np.ndarray, settings: list[dict]) -> np.ndarray:
    processed = samples - float(np.mean(samples))
    processed = evaluator.apply_polish_filters(processed, settings)
    processed *= math.sqrt(float(np.mean(np.square(samples)))) / max(
        math.sqrt(float(np.mean(np.square(processed)))), 1e-12
    )
    peak = float(np.max(np.abs(processed)))
    if peak > MAXIMUM_POLISHED_PEAK:
        processed *= MAXIMUM_POLISHED_PEAK / peak
    return processed


def update_manifest_records(manifest: dict, ledger_entries: list[dict]) -> None:
    ledger_by_asset_id = {entry["asset_id"]: entry for entry in ledger_entries}
    for collection in ("loops", "coast_loops"):
        for asset in manifest[collection]:
            entry = ledger_by_asset_id.get(asset["id"])
            if entry is None:
                continue
            asset["derived_sha256"] = entry["post_sha256"]
            asset["derived_rms_dbfs"] = entry["post_rms_dbfs"]
            asset["derived_peak_dbfs"] = entry["post_peak_dbfs"]
            asset["derived_band_rms_300_6000"] = entry["post_band_rms_300_6000"]
    manifest["polish_tool"] = TOOL_NAME
    manifest["polish_tool_revision"] = TOOL_REVISION
    selection_hashes = {entry["asset_id"]: entry["post_sha256"] for entry in ledger_entries}
    for record in manifest.get("engine_source_selection", []):
        runtime_sha256 = selection_hashes.get(record.get("runtime_asset_id"))
        if runtime_sha256 is not None:
            record["runtime_sha256"] = runtime_sha256


def update_source_inventory(
    inventory: dict,
    ledger_entries: list[dict],
    settings_by_asset_id: dict[str, list[dict]],
) -> None:
    post_hashes = {entry["asset_id"]: entry["post_sha256"] for entry in ledger_entries}
    runtime_filenames = {}
    for record in inventory["engine_sources"]:
        runtime_filename = record.get("runtime_filename")
        asset_id = record.get("runtime_asset_id")
        runtime_filenames[asset_id] = runtime_filename
        if asset_id in post_hashes:
            record["runtime_sha256"] = post_hashes[asset_id]
    polish_entries = []
    for entry in ledger_entries:
        polish_entries.append(
            {
                "asset_id": entry["asset_id"],
                "runtime_filename": runtime_filenames[entry["asset_id"]],
                "pre_sha256": entry["pre_sha256"],
                "post_sha256": entry["post_sha256"],
                "removed_direct_current": entry["removed_direct_current"],
                "post_rms_dbfs": entry["post_rms_dbfs"],
                "post_peak_dbfs": entry["post_peak_dbfs"],
                "settings": settings_by_asset_id[entry["asset_id"]],
            }
        )
    polish_entries.sort(key=lambda item: item["asset_id"])
    inventory["engine_polish"] = {
        "schema_version": 1,
        "tool": TOOL_NAME,
        "tool_revision": TOOL_REVISION,
        "direct_notes": "direct-current removal plus frozen zero-phase EQ chains; compression rejected by audit",
        "assets": polish_entries,
    }


def build_ledger(bank_directory: Path) -> tuple[list[dict], dict[str, list[dict]]]:
    assets = evaluator.evaluate_engine_assets(bank_directory)
    chain_assets = [asset for asset in assets if asset["asset_id"] in APPROVED_POLISH_CHAINS]
    if len(chain_assets) != len(APPROVED_POLISH_CHAINS):
        missing = sorted(set(APPROVED_POLISH_CHAINS) - {asset["asset_id"] for asset in chain_assets})
        raise ValueError(f"polish chains reference unknown engine loops: {missing}")
    ledger_entries = []
    for asset in chain_assets:
        asset_id = asset["asset_id"]
        samples = asset["samples"]
        destination = bank_directory / asset["derived_filename"]
        payload = destination.read_bytes()
        pre_sha256 = sha256_bytes(payload)
        processed_samples = polish_samples(samples, APPROVED_POLISH_CHAINS[asset_id])
        ledger_entries.append(
            {
                "asset_id": asset_id,
                "derived_filename": asset["derived_filename"],
                "collection": asset["collection"],
                "samples": processed_samples,
                "pre_sha256": pre_sha256,
                "post_sha256": sha256_bytes(canonical.wave_mono16_bytes(processed_samples)),
                "removed_direct_current": abs(float(np.mean(samples))) > evaluator.DIRECT_CURRENT_LIMIT,
                "post_rms_dbfs": round(
                    decibels_full_scale(math.sqrt(float(np.mean(np.square(processed_samples))))), 3
                ),
                "post_peak_dbfs": round(
                    decibels_full_scale(float(np.max(np.abs(processed_samples)))), 3
                ),
                "post_band_rms_300_6000": round(
                    canonical.band_rms(processed_samples, *canonical.LEVEL_BAND_HZ), 6
                ),
            }
        )
    return ledger_entries, APPROVED_POLISH_CHAINS


def apply_polish(bank_directory: Path) -> int:
    manifest = json.loads((bank_directory / "manifest.json").read_text(encoding="utf-8"))
    if manifest.get("polish_tool"):
        raise SystemExit(
            "the bank already declares polish provenance; inspect with --check instead of rewriting"
        )
    validate_output_path(ROOT, bank_directory / "manifest.json", "promote")
    ledger_entries, settings_by_asset_id = build_ledger(bank_directory)
    for entry in ledger_entries:
        payload = canonical.wave_mono16_bytes(entry["samples"])
        destination = bank_directory / entry["derived_filename"]
        validate_output_path(ROOT, destination, "promote")
        entry["post_sha256"] = sha256_bytes(payload)
        destination.write_bytes(payload)
        entry.pop("samples")
        entry.pop("collection")
    update_manifest_records(manifest, ledger_entries)
    inventory = json.loads(
        (bank_directory / "source_inventory.json").read_text(encoding="utf-8")
    )
    update_source_inventory(inventory, ledger_entries, settings_by_asset_id)
    inventory_bytes = canonical.serialize_json(inventory)
    (bank_directory / "source_inventory.json").write_bytes(inventory_bytes)
    manifest["source_inventory_sha256"] = sha256_bytes(inventory_bytes)
    (bank_directory / "manifest.json").write_bytes(canonical.serialize_json(manifest))
    for entry in ledger_entries:
        print(
            f"polished {entry['asset_id']}: peak {entry['post_peak_dbfs']} dBFS,"
            f" RMS {entry['post_rms_dbfs']} dBFS"
        )
    print(f"applied polish to {len(ledger_entries)} engine loops in {bank_directory}")
    return 0


def verify_polish(bank_directory: Path) -> int:
    manifest = json.loads((bank_directory / "manifest.json").read_text(encoding="utf-8"))
    if not manifest.get("polish_tool"):
        print("the bank has no polish provenance yet; run without --check to apply it")
        return 1
    inventory = json.loads(
        (bank_directory / "source_inventory.json").read_text(encoding="utf-8")
    )
    ledger = inventory.get("engine_polish")
    if ledger is None:
        print("source_inventory.json has no engine_polish ledger; refuse to continue")
        return 1
    settings_expected = APPROVED_POLISH_CHAINS
    exit_code = 0
    for entry in ledger["assets"]:
        destination = bank_directory / entry["runtime_filename"]
        payload = destination.read_bytes() if destination.is_file() else b""
        digest = sha256_bytes(payload)
        if digest != entry["post_sha256"]:
            print(f"runtime asset does not match the polish ledger: {destination.name}")
            exit_code = 1
            continue
        samples = canonical.read_wave_mono(destination)[1]
        direct_current = float(np.mean(samples))
        if abs(direct_current) > evaluator.DIRECT_CURRENT_LIMIT:
            print(f"direct-current offset survived polish: {destination.name} {direct_current}")
            exit_code = 1
        delivered = [
            {key: value for key, value in setting.items() if key != "measured"}
            for setting in entry["settings"]
        ]
        frozen = settings_expected.get(entry["asset_id"])
        if delivered != frozen:
            print(f"polish chain drifted from the approved settings: {entry['asset_id']}")
            exit_code = 1
        manifest_asset = next(
            (asset for asset in manifest["loops"] + manifest["coast_loops"]
             if asset["id"] == entry["asset_id"]),
            None,
        )
        if manifest_asset is None or manifest_asset["derived_sha256"] != digest:
            print(f"manifest waveform hash inconsistent with the ledger: {entry['asset_id']}")
            exit_code = 1
    inventory_digest = sha256_bytes((bank_directory / "source_inventory.json").read_bytes())
    if inventory_digest != manifest["source_inventory_sha256"]:
        print("source inventory digest does not match the manifest record")
        exit_code = 1
    if exit_code == 0:
        print("polish ledger verifies against every polished engine loop")
    return exit_code


def main() -> int:
    argument_parser = argparse.ArgumentParser()
    argument_parser.add_argument("--bank-directory", type=Path, default=DEFAULT_BANK_DIRECTORY)
    argument_parser.add_argument("--check", action="store_true")
    arguments = argument_parser.parse_args()
    if arguments.check:
        return verify_polish(arguments.bank_directory)
    return apply_polish(arguments.bank_directory)


if __name__ == "__main__":
    raise SystemExit(main())
