from __future__ import annotations

import json
from pathlib import Path


def load_guardrail_layout(repo: Path, config: dict) -> dict:
    relative = config.get("guardrails", {}).get("manifest")
    if not relative:
        return {"segments": []}
    layout = json.loads((repo / relative).read_text(encoding="utf-8"))
    if int(layout.get("schema_version", 0)) != 1:
        raise ValueError("Unsupported guardrail manifest schema")
    names = set()
    for segment in layout.get("segments", []):
        name = str(segment["name"])
        if name in names:
            raise ValueError(f"Duplicate guardrail segment: {name}")
        names.add(name)
        if segment["side"] not in {"left", "right"}:
            raise ValueError(f"Invalid guardrail side: {name}")
        if not (0.0 <= float(segment["start_fraction"]) < float(segment["end_fraction"]) <= 1.0):
            raise ValueError(f"Invalid guardrail fractions: {name}")
        if int(segment["rail_count"]) not in {2, 3} or float(segment["center_distance_m"]) <= 6.0:
            raise ValueError(f"Invalid guardrail geometry: {name}")
    return layout


def guardrail_conflict(layout: dict, category: str, fraction: float, side: int, distance: float, radius: float) -> bool:
    clearance = float(layout.get("vegetation_clearance_m", {}).get(category, 0.0))
    for segment in layout.get("segments", []):
        segment_side = 1 if segment["side"] == "right" else -1
        if segment_side != side:
            continue
        if float(segment["start_fraction"]) <= fraction <= float(segment["end_fraction"]):
            if abs(distance - float(segment["center_distance_m"])) < radius + clearance:
                return True
    return False
