from __future__ import annotations

import argparse
import json
from pathlib import Path

from semantic_layout_common import compile_layout


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate compiled semantic layout provenance and placement contracts.")
    parser.add_argument("--layout-config", required=True)
    parser.add_argument("--raw-config", required=True)
    args = parser.parse_args()
    layout_path = Path(args.layout_config).resolve()
    raw_path = Path(args.raw_config).resolve()
    repo = layout_path.parents[4]
    raw_config = json.loads(raw_path.read_text(encoding="utf-8"))
    compiled_path = repo / raw_config["compiled_layout"]
    if not compiled_path.exists():
        print(f"FAIL missing compiled layout: {compiled_path}")
        return 2
    stored = json.loads(compiled_path.read_text(encoding="utf-8"))
    regenerated = compile_layout(layout_path)
    failures = []
    if stored != regenerated:
        failures.append("compiled layout is stale or non-deterministic")
    ids = [item["instance_id"] for item in stored["objects"] + stored["vegetation"]]
    if len(ids) != len(set(ids)):
        failures.append("instance IDs are not unique")
    if any(item.get("collision") for item in stored["objects"] + stored["vegetation"]):
        failures.append("visual object or vegetation has collision enabled")
    outer_side = int(raw_config["outer_side"])
    for item in stored["vegetation"]:
        if "/assets_v2/glb/" not in item["asset_path"]:
            failures.append(f"legacy vegetation asset is still active: {item['instance_id']}")
        if item["category"] == "grass" and float(item["distance_from_center_m"]) > float(raw_config["zones"]["grass_max_distance_to_track_m"]) + 0.75:
            failures.append(f"grass outside playable perimeter: {item['instance_id']}")
        if item["category"] in {"trees", "bushes"} and int(item["side"]) != outer_side:
            failures.append(f"exterior vegetation on wrong side: {item['instance_id']}")
        if item["category"] in {"trees", "bushes"} and float(item.get("barrier_distance_m", 0.0)) + 1e-4 < float(item.get("required_barrier_distance_m", 0.0)):
            failures.append(f"vegetation footprint crosses barrier: {item['instance_id']}")
    offsets = {"spectator": 5.0, "marshal": 5.0, "photographer": 4.0, "sign": 2.0, "flag": 3.0}
    for item in stored["objects"]:
        expected = offsets[item["category"]]
        actual = ((float(item["position_xz"][0]) - float(item["barrier_position_xz"][0])) ** 2 + (float(item["position_xz"][1]) - float(item["barrier_position_xz"][1])) ** 2) ** 0.5
        if abs(actual - expected) > 0.02 or abs(float(item["guardrail_offset_m"]) - expected) > 1e-6:
            failures.append(f"indexed object violates guardrail offset: {item['instance_id']}")
        if int(item["side"]) != outer_side:
            failures.append(f"indexed object is not outside barrier: {item['instance_id']}")
    expected_counts = {"spectator": 42, "marshal": 17, "photographer": 13, "sign": 19, "flag": 33}
    if stored["counts"]["by_object_category"] != expected_counts:
        failures.append(f"indexed object counts changed: {stored['counts']['by_object_category']}")
    expected_vegetation = {"bushes": 110, "grass": 9320, "trees": 130}
    if stored["counts"]["by_vegetation_category"] != expected_vegetation:
        failures.append(f"vegetation counts changed: {stored['counts']['by_vegetation_category']}")
    near_grass = sum(1 for item in stored["vegetation"] if item["category"] == "grass" and item.get("density_band") == "near")
    if near_grass < 6780:
        failures.append(f"near-track grass is below 10x baseline: {near_grass}/6780")
    if failures:
        for failure in failures:
            print(f"FAIL {failure}")
        return 2
    print(f"PASS semantic layout objects={len(stored['objects'])} vegetation={len(stored['vegetation'])}")
    print(f"PASS vegetation v2 bushes=110 grass=9320 near_grass={near_grass} trees=130")
    print(f"PASS provenance semantic={stored['provenance']['semantic_sha256']} markers={stored['provenance']['marker_sha256']}")
    print("PASS collision contract: indexed objects and vegetation are visual-only")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
