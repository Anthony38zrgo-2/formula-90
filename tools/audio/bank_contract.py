"""Minimal executable contract for Formula90s audio bank manifests."""

from __future__ import annotations

import json
import re
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import Any

CONTRACT_PATH = Path("formats/audio_bank/manifest.schema.json")


@dataclass(frozen=True)
class ContractIssue:
    path: str
    code: str
    message: str


def load_contract(path: Path = CONTRACT_PATH) -> dict[str, Any]:
    """Load the repository-owned contract without a third-party dependency."""

    return json.loads(path.read_text(encoding="utf-8"))


def _is_number(value: object) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def _type_matches(value: object, expected: str) -> bool:
    return {
        "array": isinstance(value, list),
        "boolean": isinstance(value, bool),
        "integer": isinstance(value, int) and not isinstance(value, bool),
        "number": _is_number(value),
        "object": isinstance(value, Mapping),
        "string": isinstance(value, str),
    }[expected]


def validate_manifest_contract(
    manifest: object,
    contract: Mapping[str, Any] | None = None,
) -> list[ContractIssue]:
    """Validate the deliberately small subset used by the v1 contract."""

    schema = contract or load_contract()
    issues: list[ContractIssue] = []
    if not isinstance(manifest, Mapping):
        return [ContractIssue("$", "type", "manifest must be an object")]

    for field in schema["required"]:
        if field not in manifest:
            issues.append(ContractIssue(field, "required", "required field is missing"))

    properties = schema["properties"]
    for field, field_schema in properties.items():
        if field not in manifest:
            continue
        value = manifest[field]
        if "const" in field_schema and value != field_schema["const"]:
            issues.append(
                ContractIssue(field, "const", f"must equal {field_schema['const']!r}")
            )
        expected_type = field_schema.get("type")
        if isinstance(expected_type, str) and not _type_matches(value, expected_type):
            issues.append(ContractIssue(field, "type", f"must be {expected_type}"))
        if isinstance(value, str):
            if len(value) < field_schema.get("minLength", 0):
                issues.append(ContractIssue(field, "min_length", "must not be empty"))
            pattern = field_schema.get("pattern")
            if pattern and re.fullmatch(pattern, value) is None:
                issues.append(ContractIssue(field, "pattern", "has invalid characters"))

    files = manifest.get("files")
    if not isinstance(files, list):
        return issues
    if not files:
        issues.append(ContractIssue("files", "min_items", "must contain at least one file"))
        return issues

    file_schema = schema["$defs"]["file"]
    file_properties = file_schema["properties"]
    seen_files: set[str] = set()
    for index, entry in enumerate(files):
        prefix = f"files[{index}]"
        if not isinstance(entry, Mapping):
            issues.append(ContractIssue(prefix, "type", "entry must be an object"))
            continue
        for field in file_schema["required"]:
            if field not in entry:
                issues.append(ContractIssue(f"{prefix}.{field}", "required", "required field is missing"))
        for field, field_contract in file_properties.items():
            if field not in entry:
                continue
            value = entry[field]
            expected = field_contract.get("type")
            allowed_types = expected if isinstance(expected, list) else [expected]
            if expected:
                type_matches = (
                    "null" in allowed_types
                    if value is None
                    else any(item != "null" and _type_matches(value, item) for item in allowed_types)
                )
                if not type_matches:
                    issues.append(
                        ContractIssue(f"{prefix}.{field}", "type", f"must match {expected!r}")
                    )
                    continue
            if isinstance(value, str):
                if len(value) < field_contract.get("minLength", 0):
                    issues.append(ContractIssue(f"{prefix}.{field}", "min_length", "must not be empty"))
                pattern = field_contract.get("pattern")
                if pattern and re.fullmatch(pattern, value) is None:
                    issues.append(ContractIssue(f"{prefix}.{field}", "pattern", "has invalid format"))
            if _is_number(value):
                if "exclusiveMinimum" in field_contract and value <= field_contract["exclusiveMinimum"]:
                    issues.append(ContractIssue(f"{prefix}.{field}", "minimum", "must be positive"))
                if "minimum" in field_contract and value < field_contract["minimum"]:
                    issues.append(ContractIssue(f"{prefix}.{field}", "minimum", "must not be negative"))

        filename = entry.get("file")
        if isinstance(filename, str):
            if filename in seen_files:
                issues.append(ContractIssue(f"{prefix}.file", "duplicate", "file must be unique"))
            seen_files.add(filename)

        playback = entry.get("playback")
        if isinstance(playback, Mapping):
            native_rpm = playback.get("native_rpm")
            if not _is_number(native_rpm) or native_rpm <= 0:
                issues.append(
                    ContractIssue(f"{prefix}.playback.native_rpm", "playback", "must be positive")
                )
            engine_band = playback.get("engine_band")
            if engine_band is not None:
                if not isinstance(engine_band, Mapping):
                    issues.append(
                        ContractIssue(f"{prefix}.playback.engine_band", "type", "must be an object")
                    )
                else:
                    band_index = engine_band.get("index")
                    center = engine_band.get("center")
                    width = engine_band.get("width")
                    if not isinstance(band_index, int) or isinstance(band_index, bool) or band_index < 0:
                        issues.append(
                            ContractIssue(f"{prefix}.playback.engine_band.index", "playback", "must be a non-negative integer")
                        )
                    if not _is_number(center) or not 0 <= center <= 1:
                        issues.append(
                            ContractIssue(f"{prefix}.playback.engine_band.center", "playback", "must be between 0 and 1")
                        )
                    if not _is_number(width) or not 0 < width <= 1:
                        issues.append(
                            ContractIssue(f"{prefix}.playback.engine_band.width", "playback", "must be within (0, 1]")
                        )

        start = entry.get("loop_start_s")
        end = entry.get("loop_end_s")
        duration = entry.get("duration_s")
        if (
            start is not None
            and end is not None
            and all(_is_number(item) for item in (start, end, duration))
            and not 0 <= start < end <= duration
        ):
            issues.append(
                ContractIssue(
                    prefix,
                    "loop_bounds",
                    "loop bounds must satisfy 0 <= start < end <= duration_s",
                )
            )
    return issues
