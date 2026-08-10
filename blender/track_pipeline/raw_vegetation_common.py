from __future__ import annotations

import hashlib


def signed_area(points) -> float:
    return 0.5 * sum(
        float(points[i][0]) * float(points[(i + 1) % len(points)][1])
        - float(points[(i + 1) % len(points)][0]) * float(points[i][1])
        for i in range(len(points))
    )


def outer_side(points) -> int:
    return 1 if signed_area(points) >= 0.0 else -1


def deterministic_index(key: str, count: int) -> int:
    digest = hashlib.sha256(key.encode("utf-8")).digest()
    return int.from_bytes(digest[:4], "big") % max(1, count)


def shift_to_distance(sample_centerline, points, item, target_distance: float, side: int):
    pos, _, normal = sample_centerline(points, float(item["track_fraction"]))
    return [
        round(pos[0] + normal[0] * side * target_distance, 4),
        round(pos[1] + normal[1] * side * target_distance, 4),
    ]


def derive_raw_placements(config, center, placements, sample_centerline):
    points = center["points_xz"]
    barrier_center = float(config["road_width_m"]) * 0.5 + float(config["barrier"]["distance_from_edge_m"])
    grass_limit = float(config["zones"]["grass_max_distance_to_track_m"])
    exterior_min = float(config["zones"]["exterior_min_distance_to_track_m"])
    exterior_side = outer_side(points)
    raw = []
    for index, item in enumerate(placements.get("placements", [])):
        category = str(item["category"])
        if category == "grass":
            if float(item.get("distance_to_track_m", 0.0)) > grass_limit:
                continue
            out = dict(item)
        elif category in {"trees", "bushes"}:
            out = dict(item)
            out["side"] = exterior_side
            if float(item.get("distance_to_track_m", 0.0)) < exterior_min:
                out["distance_from_center_m"] = round(exterior_min + float(config["road_width_m"]) * 0.5, 4)
                out["position_xz"] = shift_to_distance(sample_centerline, points, out, out["distance_from_center_m"], exterior_side)
            else:
                out["position_xz"] = list(item["position_xz"])
        else:
            continue
        out["raw_pipeline"] = True
        out["raw_index"] = index
        raw.append(out)
    props = []
    prop_distance = barrier_center + float(config["barrier"]["outside_prop_offset_m"])
    for index, item in enumerate(placements.get("trackside_props", [])):
        out = dict(item)
        out["side"] = exterior_side
        out["distance_from_center_m"] = round(prop_distance, 4)
        out["position_xz"] = shift_to_distance(sample_centerline, points, out, prop_distance, exterior_side)
        out["collision"] = False
        out["raw_pipeline"] = True
        out["raw_index"] = index
        props.append(out)
    return {"schema_version": 1, "barrier_center_m": barrier_center, "outer_side": exterior_side, "placements": raw, "trackside_props": props}
