"""Deterministic, evidence-based semantic suggestions for vehicle scans.

Every result requires explicit human confirmation before it can enter a
VehicleDocument; this module never creates authoritative mappings.
"""
from __future__ import annotations

import re
from typing import Any, Iterable

from .canonical import canonical_json_bytes

_NAME_RULES: tuple[tuple[re.Pattern[str], str, str], ...] = (
    (re.compile(r"(?:^|_)FRONT_WING(?:_|$)"), "component", "front_wing"),
    (re.compile(r"(?:^|_)REAR_WING(?:_|$)"), "component", "rear_wing"),
    (re.compile(r"(?:^|_)SIDEPOD(?:S)?(?:_|$)"), "component", "sidepods"),
    (re.compile(r"(?:^|_)NOSE(?:_|$)"), "component", "nose"),
    (re.compile(r"(?:^|_)(?:ENGINE|MOTOR)_COVER(?:_|$)"), "component", "engine_cover"),
    (re.compile(r"(?:^|_)CHASSIS(?:_|$)"), "component", "chassis"),
    (re.compile(r"(?:^|_)COCKPIT(?:_|$)"), "component", "cockpit"),
    (re.compile(r"(?:^|_)DRIVER(?:_|$)"), "component", "driver"),
    (re.compile(r"(?:^|_)SUSPENSION_(FL|FR|RL|RR)(?:_|$)"), "component", "suspension_{corner}"),
    (re.compile(r"(?:^|_)(?:WHEEL|TYRE|TIRE)_(FL|FR|RL|RR)(?:_|$)"), "component", "wheel_{corner}"),
    (re.compile(r"^(?:DATUM|JNT)_(FL|FR|RL|RR)(?:_|$)"), "frame", "wheel_{corner}_anchor"),
    (re.compile(r"^(?:DATUM|JNT)_(?:VEHICLE_)?ORIGIN(?:_|$)"), "frame", "vehicle_origin"),
)


def _names_from_scan(scan: dict[str, Any]) -> Iterable[tuple[str, str]]:
    evidence = scan.get("evidence", {})
    for collection, source_kind, keys in (
        ("objects", "object", ("name",)),
        ("nodes_detail", "node", ("name",)),
        ("primitive_attribute_coverage", "mesh", ("mesh_name", "name")),
    ):
        for item in evidence.get(collection, []):
            name = next((item.get(key) for key in keys if item.get(key)), None)
            if isinstance(name, str):
                yield source_kind, name


def _match(name: str) -> tuple[str, str, str] | None:
    normalized = re.sub(r"[^A-Z0-9]+", "_", name.upper()).strip("_")
    for pattern, kind, role_template in _NAME_RULES:
        matched = pattern.search(normalized)
        if matched:
            corner = matched.group(1).lower() if matched.lastindex else ""
            return kind, role_template.replace("{corner}", corner), normalized
    return None


def suggest_semantics(scan: dict[str, Any]) -> dict[str, Any]:
    source = scan.get("source", {})
    suggestions: list[dict[str, Any]] = []
    seen: set[tuple[str, str, str]] = set()
    for source_kind, name in _names_from_scan(scan):
        match = _match(name)
        if match is None:
            continue
        semantic_kind, proposed_role, normalized = match
        key = (source_kind, name, proposed_role)
        if key in seen:
            continue
        seen.add(key)
        suggestions.append({
            "confidence": "high",
            "evidence": [{
                "code": "explicit_name_token",
                "normalized_name": normalized,
                "source_name": name,
            }],
            "proposed_role": proposed_role,
            "requires_human_confirmation": True,
            "semantic_kind": semantic_kind,
            "source_kind": source_kind,
            "source_name": name,
        })
    suggestions.sort(key=lambda item: (
        item["semantic_kind"], item["proposed_role"],
        item["source_kind"], item["source_name"],
    ))
    for index, suggestion in enumerate(suggestions, start=1):
        suggestion["suggestion_id"] = f"suggestion-{index:04d}"
    return {
        "adapter": scan.get("adapter"),
        "authority": "suggestion_only",
        "schema_version": "vehicle-semantic-suggestions/v1",
        "source": {"path": source.get("path"), "sha256": source.get("sha256")},
        "suggestions": suggestions,
    }


def semantic_suggestions_bytes(scan: dict[str, Any]) -> bytes:
    return canonical_json_bytes(suggest_semantics(scan))
