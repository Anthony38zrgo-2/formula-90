#!/usr/bin/env python3
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
import shutil
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SOURCE_DIRECTORY = Path(r"D:\ASETS\f12002")
DEFAULT_BASE_BANK_DIRECTORY = ROOT / "game/audio/formula_one_2030_grand_prix_sampler"
DEFAULT_OUTPUT_BANK_DIRECTORY = ROOT / "game/audio/formula_one_2030_grand_prix_sampler/formula_one_2030_five_engine_sample_bank"
OUTPUT_SAMPLE_RATE = 44100
TRANSITION_HALF_WIDTH_RATIO = 0.075

ENGINE_SOURCE_SELECTION = [
    {
        "role": "engine_idle",
        "source_filename": "idle.wav",
        "source_sha256": "a2af839a00955f414b29e27fcae00837a00ca8b4ad1633dac75880a82cc00101",
        "base_collection": "loops",
        "base_asset_id": "engine_idle_loop",
        "asset_id": "engine_idle_loop",
        "derived_filename": "engine_idle_loop.wav",
    },
    {
        "role": "engine_low_on",
        "source_filename": "low_on.wav",
        "source_sha256": "9dccccec035b1553824ae815761b9893891126f06afb9133a7abf3add6faaae4",
        "base_collection": "loops",
        "base_asset_id": "engine_low_loop",
        "asset_id": "engine_low_on_loop",
        "derived_filename": "engine_low_on_loop.wav",
    },
    {
        "role": "engine_high_on",
        "source_filename": "hi_max_on.wav",
        "source_sha256": "638485f34e50b1f4df46a406acf548eb22448ce208cf40540408cd466b6f638b",
        "base_collection": "loops",
        "base_asset_id": "engine_high_loop",
        "asset_id": "engine_high_on_loop",
        "derived_filename": "engine_high_on_loop.wav",
    },
    {
        "role": "engine_low_off",
        "source_filename": "low_off.wav",
        "source_sha256": "a3ae253ddeaf19872af315305a2c54b7cc6025dba7a8efad7440bf4acecd9c55",
        "base_collection": "coast_loops",
        "base_asset_id": "engine_coast_low_loop",
        "asset_id": "engine_low_off_loop",
        "derived_filename": "engine_low_off_loop.wav",
    },
    {
        "role": "engine_high_off",
        "source_filename": "hi_off.wav",
        "source_sha256": "4142964333dba9aae06a20d76827bc01e0956b8401b984179a23a347ec4d3c3c",
        "base_collection": "coast_loops",
        "base_asset_id": "engine_coast_high_loop",
        "asset_id": "engine_high_off_loop",
        "derived_filename": "engine_high_off_loop.wav",
    },
]


def sha256_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source_file:
        for chunk in iter(lambda: source_file.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def serialize_json(payload: object) -> bytes:
    return (json.dumps(payload, ensure_ascii=False, indent=2) + "\n").encode("utf-8")


def selected_base_asset(base_manifest: dict, selection: dict) -> dict:
    assets = base_manifest[selection["base_collection"]]
    matches = [asset for asset in assets if asset["id"] == selection["base_asset_id"]]
    if len(matches) != 1:
        raise ValueError(f"Expected one base asset for {selection['base_asset_id']}; found {len(matches)}")
    return copy.deepcopy(matches[0])


def transition(from_asset_id: str, to_asset_id: str, start: float, end: float, measured_compensation: float) -> dict:
    center = (start + end) * 0.5
    return {
        "from_loop_id": from_asset_id,
        "to_loop_id": to_asset_id,
        "start_revolutions_per_minute": start,
        "end_revolutions_per_minute": end,
        "center_revolutions_per_minute": center,
        "half_width_revolutions_per_minute": (end - start) * 0.5,
        "blend_law": "smoothstep",
        "gain_law": "equal_power",
        "measured_level_compensation_db": measured_compensation,
    }


def set_loop_coverage(assets: list[dict], transitions: list[dict], coverage: dict) -> None:
    for index, asset in enumerate(assets):
        start = coverage["minimum_revolutions_per_minute"] if index == 0 else transitions[index - 1]["start_revolutions_per_minute"]
        end = coverage["maximum_revolutions_per_minute"] if index + 1 == len(assets) else transitions[index]["end_revolutions_per_minute"]
        reference = asset["reference_revolutions_per_minute"]
        asset["active_coverage_revolutions_per_minute"] = [start, end]
        asset["valid_playback_rate_min"] = min(asset["valid_playback_rate_min"], start / reference)
        asset["valid_playback_rate_max"] = max(asset["valid_playback_rate_max"], end / reference)


def prepare_bank(source_directory: Path, base_bank_directory: Path) -> tuple[dict, dict[str, Path]]:
    base_manifest_path = base_bank_directory / "manifest.json"
    base_manifest = json.loads(base_manifest_path.read_text(encoding="utf-8"))
    output_assets: dict[str, Path] = {}
    selected_loops = []
    selected_coast_loops = []
    inventory_engine_sources = []

    for selection in ENGINE_SOURCE_SELECTION:
        source_path = source_directory / selection["source_filename"]
        if not source_path.is_file():
            raise FileNotFoundError(f"Missing original F12002 source: {source_path}")
        source_hash = sha256_file(source_path)
        if source_hash != selection["source_sha256"]:
            raise ValueError(f"Original source hash mismatch: {source_path}")

        base_asset = selected_base_asset(base_manifest, selection)
        base_asset_path = base_bank_directory / base_asset["derived_filename"]
        if sha256_file(base_asset_path) != base_asset["derived_sha256"]:
            raise ValueError(f"Base prepared loop hash mismatch: {base_asset_path}")

        active_asset = copy.deepcopy(base_asset)
        active_asset["id"] = selection["asset_id"]
        active_asset["derived_filename"] = selection["derived_filename"]
        active_asset["original_source_filename"] = selection["source_filename"]
        active_asset["original_source_path"] = source_path.as_posix()
        active_asset["original_source_sha256"] = source_hash
        active_asset["role"] = "engine_loop" if selection["base_collection"] == "loops" else "engine_coast_loop"
        active_asset["source_filename"] = base_asset["source_filename"]
        active_asset["source_sha256"] = base_asset["source_sha256"]

        if selection["base_collection"] == "loops":
            selected_loops.append(active_asset)
        else:
            selected_coast_loops.append(active_asset)

        output_assets[selection["derived_filename"]] = base_asset_path
        inventory_engine_sources.append(
            {
                "role": selection["role"],
                "source_path": source_path.as_posix(),
                "source_sha256": source_hash,
                "prepared_bank_asset_id": base_asset["id"],
                "prepared_source_filename": base_asset["source_filename"],
                "prepared_source_sha256": base_asset["source_sha256"],
                "runtime_asset_id": selection["asset_id"],
                "runtime_filename": selection["derived_filename"],
                "runtime_sha256": base_asset["derived_sha256"],
            }
        )

    selected_loops.sort(key=lambda asset: asset["reference_revolutions_per_minute"])
    selected_coast_loops.sort(key=lambda asset: asset.get("zone_revolutions_per_minute", asset["reference_revolutions_per_minute"]))
    coverage = copy.deepcopy(base_manifest["coverage"])

    original_first_transition = next(
        item
        for item in base_manifest["transitions"]
        if item["from_loop_id"] == "engine_idle_loop" and item["to_loop_id"] == "engine_low_loop"
    )
    first_transition = transition(
        selected_loops[0]["id"],
        selected_loops[1]["id"],
        original_first_transition["start_revolutions_per_minute"],
        original_first_transition["end_revolutions_per_minute"],
        original_first_transition.get("measured_level_compensation_db", 0.0),
    )
    second_center = math.sqrt(
        selected_loops[1]["reference_revolutions_per_minute"]
        * selected_loops[2]["reference_revolutions_per_minute"]
    )
    second_half_width = second_center * TRANSITION_HALF_WIDTH_RATIO
    second_transition = transition(
        selected_loops[1]["id"],
        selected_loops[2]["id"],
        second_center - second_half_width,
        second_center + second_half_width,
        0.0,
    )
    powered_transitions = [first_transition, second_transition]
    set_loop_coverage(selected_loops, powered_transitions, coverage)

    original_coast_transition = base_manifest["coast_transitions"][0]
    coast_transition = transition(
        selected_coast_loops[0]["id"],
        selected_coast_loops[1]["id"],
        original_coast_transition["start_revolutions_per_minute"],
        original_coast_transition["end_revolutions_per_minute"],
        original_coast_transition.get("measured_level_compensation_db", 0.0),
    )
    set_loop_coverage(selected_coast_loops, [coast_transition], coverage)

    preserved_events = copy.deepcopy(base_manifest["events"])
    inventory_preserved_events = []
    for event_asset in preserved_events:
        event_path = base_bank_directory / event_asset["derived_filename"]
        if sha256_file(event_path) != event_asset["derived_sha256"]:
            raise ValueError(f"Preserved non-engine event hash mismatch: {event_path}")
        output_assets[event_asset["derived_filename"]] = event_path
        inventory_preserved_events.append(
            {
                "asset_id": event_asset["id"],
                "role": event_asset["role"],
                "runtime_filename": event_asset["derived_filename"],
                "runtime_sha256": event_asset["derived_sha256"],
            }
        )

    source_inventory = {
        "bank_id": "formula_one_2030_five_engine_sample_bank",
        "engine_source_directory": source_directory.as_posix(),
        "engine_sources": inventory_engine_sources,
        "preserved_non_engine_events": inventory_preserved_events,
    }
    inventory_bytes = serialize_json(source_inventory)
    manifest = {
        "schema_version": base_manifest["schema_version"],
        "bank_id": source_inventory["bank_id"],
        "preparation_tool": "scripts/audio/prepare_formula_one_2030_five_engine_sample_bank.py",
        "preparation_tool_revision": 1,
        "source_inventory_sha256": sha256_bytes(inventory_bytes),
        "output_sample_rate": OUTPUT_SAMPLE_RATE,
        "event_selection_seed": base_manifest["event_selection_seed"],
        "coverage": coverage,
        "loops": selected_loops,
        "transitions": powered_transitions,
        "coast_loops": selected_coast_loops,
        "coast_transitions": [coast_transition],
        "events": preserved_events,
        "event_groups": copy.deepcopy(base_manifest["event_groups"]),
        "event_trigger_policy": copy.deepcopy(base_manifest["event_trigger_policy"]),
        "engine_source_selection": inventory_engine_sources,
    }
    manifest["_source_inventory_bytes"] = inventory_bytes
    return manifest, output_assets


def main() -> int:
    argument_parser = argparse.ArgumentParser()
    argument_parser.add_argument("--source-directory", type=Path, default=DEFAULT_SOURCE_DIRECTORY)
    argument_parser.add_argument("--base-bank-directory", type=Path, default=DEFAULT_BASE_BANK_DIRECTORY)
    argument_parser.add_argument("--output-bank-directory", type=Path, default=DEFAULT_OUTPUT_BANK_DIRECTORY)
    argument_parser.add_argument("--check", action="store_true")
    arguments = argument_parser.parse_args()

    manifest, output_assets = prepare_bank(arguments.source_directory, arguments.base_bank_directory)
    inventory_bytes = manifest.pop("_source_inventory_bytes")
    manifest_bytes = serialize_json(manifest)
    output_files = {**output_assets, "source_inventory.json": None, "manifest.json": None}
    expected_contents = {
        filename: source_path.read_bytes() if source_path is not None else None
        for filename, source_path in output_files.items()
    }
    expected_contents["source_inventory.json"] = inventory_bytes
    expected_contents["manifest.json"] = manifest_bytes

    if arguments.check:
        if not arguments.output_bank_directory.is_dir():
            raise FileNotFoundError(f"Prepared variant bank is missing: {arguments.output_bank_directory}")
        for filename, expected_bytes in expected_contents.items():
            output_path = arguments.output_bank_directory / filename
            if not output_path.is_file() or output_path.read_bytes() != expected_bytes:
                raise ValueError(f"Prepared bank file differs from its reproducible source: {output_path}")
        print(f"Verified {len(output_assets)} WAV assets and deterministic manifests in {arguments.output_bank_directory}")
        return 0

    arguments.output_bank_directory.mkdir(parents=True, exist_ok=True)
    for filename, expected_bytes in expected_contents.items():
        output_path = arguments.output_bank_directory / filename
        output_path.write_bytes(expected_bytes)
    print(f"Prepared {len(output_assets)} WAV assets and a five-sample engine variant in {arguments.output_bank_directory}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
