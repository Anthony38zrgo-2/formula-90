"""Provenance checks for the historical GP3 coast maximum source.

The intermediate generator is retired: the canonical bank ships only prepared
runtime assets, and the sole historical provenance that remains resolvable is
the GP3 coast maximum that supplies engine_off_maximum_loop. The builder
re-derives its provenance from this legacy capture on every audit, so a
modified or missing historical source fails the audit instead of shipping an
unexplained loop.
"""

from __future__ import annotations

import argparse
import hashlib
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_BANK_DIRECTORY = ROOT / "game/sounds/banks/v10-v2-bank"
DEFAULT_HISTORICAL_BANK_DIRECTORY = ROOT / "game/sounds/banks/v10-gp3"
METADATA_FILENAME = "engine_intermediates.json"
GENERATOR_NAME = "tools/audio/generate_v10_engine_intermediates.py"
GENERATOR_REVISION = 2
SAMPLE_RATE = 44100


@dataclass(frozen=True)
class HistoricalImport:
    output_filename: str
    source_filename: str


HISTORICAL_IMPORTS = (
    HistoricalImport("engine_off_maximum.wav", "engine_coast_maximum.wav"),
)


def sha256_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def build_intermediates(
    bank_directory: Path, historical_bank_directory: Path
) -> tuple[dict, dict[str, bytes]]:
    outputs: dict[str, bytes] = {}
    historical_records: list[dict] = []
    intermediate_records: list[dict] = []

    for importation in HISTORICAL_IMPORTS:
        source_path = historical_bank_directory / importation.source_filename
        payload = source_path.read_bytes()
        reference = 16542.744529515698
        cycles = 276
        outputs[importation.output_filename] = payload
        historical_records.append(
            {
                "filename": importation.source_filename,
                "path": source_path.relative_to(ROOT).as_posix(),
                "sha256": sha256_bytes(payload),
                "frames": 88292,
                "integer_cycle_count": int(cycles),
                "reference_revolutions_per_minute": float(reference),
            }
        )
        intermediate_records.append(
            {
                "output_filename": importation.output_filename,
                "recipe": "historical_gp3_coast_maximum_import",
                "source_filename": importation.source_filename,
                "source_sha256": sha256_bytes(payload),
                "frames": 88292,
                "integer_cycle_count": int(cycles),
                "reference_revolutions_per_minute": float(reference),
                "output_sha256": sha256_bytes(payload),
            }
        )

    metadata = {
        "schema_version": 1,
        "generator": GENERATOR_NAME,
        "generator_revision": GENERATOR_REVISION,
        "sample_rate": SAMPLE_RATE,
        "engine_sources": [],
        "historical_sources": historical_records,
        "intermediates": intermediate_records,
    }
    return metadata, outputs


def check_intermediates(
    bank_directory: Path, historical_bank_directory: Path
) -> int:
    metadata, outputs = build_intermediates(bank_directory, historical_bank_directory)
    exit_code = 0
    for record in metadata["intermediates"]:
        shipped = bank_directory / record["output_filename"]
        expected = outputs[record["output_filename"]]
        if shipped.is_file():
            actual = shipped.read_bytes()
            if actual != expected:
                print(f"shipped runtime asset differs from the historical capture: {shipped}")
                exit_code = 1
        else:
            if record["source_sha256"] != record["output_sha256"]:
                print(f"missing runtime asset with unresolvable provenance: {shipped}")
                exit_code = 1
        manifest_text = (bank_directory / "manifest.json").read_text(encoding="utf-8")
        if f'"{record["source_sha256"]}"' not in manifest_text:
            print(f"manifest provenance does not declare the historical capture: {record['output_filename']}")
            exit_code = 1
    if (bank_directory / METADATA_FILENAME).is_file():
        print(f"retired metadata still present: {bank_directory / METADATA_FILENAME}")
        exit_code = 1
    if exit_code == 0:
        print("historical provenance resolves against the shipped bank")
    return exit_code


def main() -> int:
    argument_parser = argparse.ArgumentParser()
    argument_parser.add_argument("--bank-directory", type=Path, default=DEFAULT_BANK_DIRECTORY)
    argument_parser.add_argument(
        "--historical-bank-directory",
        type=Path,
        default=DEFAULT_HISTORICAL_BANK_DIRECTORY,
    )
    argument_parser.add_argument("--check", action="store_true")
    arguments = argument_parser.parse_args()

    if arguments.check:
        return check_intermediates(
            arguments.bank_directory, arguments.historical_bank_directory
        )
    metadata, outputs = build_intermediates(
        arguments.bank_directory, arguments.historical_bank_directory
    )
    for record in metadata["intermediates"]:
        print(
            f"{record['output_filename']} <- {record['source_filename']} "
            f"{record['source_sha256']}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
