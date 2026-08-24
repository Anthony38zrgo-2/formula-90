




"""Pure compiler from accepted onboarding mapping to VehicleDocument revision zero."""
from __future__ import annotations

from copy import deepcopy
from hashlib import sha256
import re
from typing import Any

from .canonical import canonical_json_bytes
from .domain import VehicleDocument
from .onboarding import OnboardingError


def compile_initial_revision(
    onboarding: dict[str, Any], *, project_id: str
) -> VehicleDocument:
    if not onboarding.get("ready_to_compile") or onboarding.get("missing_requirements"):
        raise OnboardingError("ONBOARDING_INCOMPLETE", "accepted mapping is incomplete")
    if not re.fullmatch(r"[a-z][a-z0-9]*(?:[._-][a-z0-9]+)*", project_id):
        raise ValueError("project_id has invalid format")

    semantic = _semantic_mapping(onboarding)
    mapping_hash = sha256(canonical_json_bytes(semantic)).hexdigest().upper()
    source = semantic["source"]
    files = source.get("files", [])
    dimensions = _dimensions(semantic["frames"], source.get("baseline_dimensions", {}))
    document = {
        "schema_version": 1,
        "project_id": project_id,
        "revision_id": f"revision-zero-{mapping_hash[:12].lower()}",
        "source": {
            "kind": "glb_bundle",
            "path": source["path"],
            "size_bytes": sum(int(item.get("size_bytes", 0)) for item in files),
            "sha256": source["sha256"],
            "evaluated_geometry_sha256": source["sha256"],
            "blender_version": None,
            "provenance_status": "HUMAN_ACCEPTED_MAPPING",
        },
        "coordinate_system": {
            "unit": "meter",
            **semantic["axes"],
            "ground_y_m": semantic["ground_y_m"],
        },
        "component_map": _components(semantic["components"]),
        "frames": _frames(semantic["frames"]),
        "dimensions": dimensions,
        "stations": [],
        "deform_regions": [],
        "parameters": _parameters(dimensions),
        "material_regions": [],
        "material_recipes": [],
        "livery_layers": [],
        "view_definitions": _views(),
        "validation_policy": {
            "topology_changes_allowed": False,
            "symmetry_default": True,
            "ground_contact_required": True,
            "uv_preservation_required": True,
            "source_mutation_allowed": False,
            "tolerance_m": 0.000001,
        },
        "metadata": {
            "vehicle_class": "formula_1_open_wheel",
            "mapping_sha256": mapping_hash,
            "mapping_status": "human_accepted",
            "symmetry_plane_x_m": semantic["symmetry_plane_x_m"],
            "materialization_source": deepcopy(source.get("materialization_source")),
            "materialization_sources": deepcopy(source.get("files", [])),
        },
    }
    return VehicleDocument.from_dict(document)


def _semantic_mapping(onboarding: dict[str, Any]) -> dict[str, Any]:
    result = {
        key: deepcopy(onboarding[key])
        for key in (
            "source", "axes", "ground_y_m", "symmetry_plane_x_m",
            "components", "frames", "optional_roles",
        )
    }
    result["components"].sort(key=lambda item: item["semantic_role"])
    result["frames"].sort(key=lambda item: item["semantic_role"])
    return result


def _components(items: list[dict[str, Any]]) -> list[dict[str, Any]]:
    results = []
    for item in items:
        role = item["semantic_role"]
        source_path = item.get("provenance", {}).get("source_path")
        source_name = item["source_name"]
        results.append({
            "component_id": f"component-{role.replace('_', '-')}",
            "semantic_role": role,
            "required_for_runtime": role not in {"cockpit", "driver"},
            "source_binding": {
                "source_path": source_path,
                "object_ids": [_slug(source_name)],
                "primitive_ids": [],
            },
            "deform_region_ids": [],
            "material_region_ids": [],
        })
    return results


def _frames(items: list[dict[str, Any]]) -> list[dict[str, Any]]:
    by_role = {item["semantic_role"]: item for item in items}
    absolute = {role: _translation(item) for role, item in by_role.items()}
    role_map = {
        "vehicle_origin": ("vehicle_origin", None),
        "front_axle_center": ("front_axle_center", "frame-vehicle-origin"),
        "rear_axle_center": ("rear_axle_center", "frame-vehicle-origin"),
        "wheel_fl_anchor": ("wheel_fl", "frame-front-axle-center"),
        "wheel_fr_anchor": ("wheel_fr", "frame-front-axle-center"),
        "wheel_rl_anchor": ("wheel_rl", "frame-rear-axle-center"),
        "wheel_rr_anchor": ("wheel_rr", "frame-rear-axle-center"),
        "front_wing_mount": ("front_wing_mount", "frame-vehicle-origin"),
        "rear_wing_mount": ("rear_wing_mount", "frame-vehicle-origin"),
    }
    results = []
    for source_role, (document_role, parent_id) in role_map.items():
        translation = absolute.get(source_role, [0.0, 0.0, 0.0])
        if source_role.startswith("wheel_"):
            axle_role = "front_axle_center" if source_role in {"wheel_fl_anchor", "wheel_fr_anchor"} else "rear_axle_center"
            axle = absolute[axle_role]
            translation = [translation[i] - axle[i] for i in range(3)]
        results.append({
            "frame_id": f"frame-{document_role.replace('_', '-')}",
            "semantic_role": document_role,
            "parent_frame_id": parent_id,
            "transform": {
                "translation_m": translation,
                "rotation_xyzw": [0, 0, 0, 1],
                "scale": [1, 1, 1],
            },
            "source_binding": {
                "source_name": by_role[source_role]["source_name"],
                "source_path": by_role[source_role].get("provenance", {}).get("source_path"),
            },
            "confirmation_status": "HUMAN_ACCEPTED",
        })
    return results


def _translation(item: dict[str, Any]) -> list[float]:
    for evidence in item.get("provenance", {}).get("evidence", []):
        value = evidence.get("translation_m")
        if isinstance(value, list) and len(value) == 3:
            return [float(number) for number in value]
    return [0.0, 0.0, 0.0]


def _dimensions(items: list[dict[str, Any]], baseline: dict[str, Any]) -> dict[str, float]:
    absolute = {item["semantic_role"]: _translation(item) for item in items}
    front = absolute["front_axle_center"]
    rear = absolute["rear_axle_center"]
    return {
        "wheelbase_m": rear[2] - front[2],
        "front_track_m": absolute["wheel_fr_anchor"][0] - absolute["wheel_fl_anchor"][0],
        "rear_track_m": absolute["wheel_rr_anchor"][0] - absolute["wheel_rl_anchor"][0],
        **{key: float(value) for key, value in baseline.items()},
    }


def _parameters(dimensions: dict[str, float]) -> list[dict[str, Any]]:
    definitions = (
        ("wheelbase", "wheelbase_m", 0.85, 1.15, [
            {"kind": "translate_axle_frames", "inputs": {"ownership": "front_and_rear_axles"}, "postconditions": ["wheelbase_matches", "wheel_anchors_match"]},
            {"kind": "piecewise_body_deform", "inputs": {"protected_frames": ["front_wing_mount", "rear_wing_mount"]}, "postconditions": ["body_continuity_matches", "topology_matches_source"]},
        ]),
        ("front_track", "front_track_m", 0.85, 1.15, [
            {"kind": "translate_wheel_frames", "inputs": {"axle": "front"}, "postconditions": ["front_track_matches", "wheel_anchors_match"]},
        ]),
        ("rear_track", "rear_track_m", 0.85, 1.15, [
            {"kind": "translate_wheel_frames", "inputs": {"axle": "rear"}, "postconditions": ["rear_track_matches", "wheel_anchors_match"]},
        ]),
        ("front_tire_radius", "front_tire_radius_m", 0.80, 1.20, [
            {"kind": "scale_tire_radial", "inputs": {"axle": "front"}, "postconditions": ["front_tire_radius_matches", "topology_matches_source"]},
            {"kind": "maintain_ground_contact", "inputs": {"axle": "front"}, "postconditions": ["ground_contact_matches", "chassis_height_reported"]},
        ]),
        ("rear_tire_radius", "rear_tire_radius_m", 0.80, 1.20, [
            {"kind": "scale_tire_radial", "inputs": {"axle": "rear"}, "postconditions": ["rear_tire_radius_matches", "topology_matches_source"]},
            {"kind": "maintain_ground_contact", "inputs": {"axle": "rear"}, "postconditions": ["ground_contact_matches", "chassis_height_reported"]},
        ]),
        ("front_tire_width", "front_tire_width_m", 0.70, 1.30, [
            {"kind": "scale_tire_axial", "inputs": {"axle": "front", "width_policy": "centered"}, "postconditions": ["front_tire_width_matches", "wheel_anchor_unchanged"]},
        ]),
        ("rear_tire_width", "rear_tire_width_m", 0.70, 1.30, [
            {"kind": "scale_tire_axial", "inputs": {"axle": "rear", "width_policy": "centered"}, "postconditions": ["rear_tire_width_matches", "wheel_anchor_unchanged"]},
        ]),
    )
    results = []
    for role, dimension, lower_factor, upper_factor, operations in definitions:
        if dimension not in dimensions:
            continue
        baseline = dimensions[dimension]
        results.append({
            "parameter_id": f"parameter-{role.replace('_', '-')}",
            "semantic_role": role,
            "unit": "meter",
            "minimum": baseline * lower_factor,
            "maximum": baseline * upper_factor,
            "baseline_value": baseline,
            "absolute_value": baseline,
            "percentage_value": 100.0,
            "dependency_ids": [],
            "build_operations": operations,
        })
    return results


def _views() -> list[dict[str, Any]]:
    return [
        {"view_id": "view-top", "kind": "top", "horizontal_axis": "+X", "vertical_axis": "-Z", "depth_axis": "+Y", "units": "meter"},
        {"view_id": "view-side", "kind": "side", "horizontal_axis": "-Z", "vertical_axis": "+Y", "depth_axis": "+X", "units": "meter"},
        {"view_id": "view-front", "kind": "front", "horizontal_axis": "+X", "vertical_axis": "+Y", "depth_axis": "+Z", "units": "meter"},
        {"view_id": "view-rear", "kind": "rear", "horizontal_axis": "-X", "vertical_axis": "+Y", "depth_axis": "-Z", "units": "meter"},
    ]


def _slug(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", value.lower()).strip("-")
