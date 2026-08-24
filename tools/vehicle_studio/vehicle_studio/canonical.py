"""Canonical VehicleDocument normalization and JSON byte serialization."""

from __future__ import annotations

from copy import deepcopy
import json
import math
from typing import Any


class CanonicalJSONError(ValueError):
    pass


_COLLECTION_KEYS: dict[str, tuple[str, ...]] = {
    "component_map": ("component_id",),
    "frames": ("frame_id",),
    "stations": ("longitudinal_position_m", "station_id"),
    "deform_regions": ("region_id",),
    "parameters": ("parameter_id",),
    "material_regions": ("region_id",),
    "material_recipes": ("recipe_id",),
    "livery_layers": ("order", "layer_id"),
    "view_definitions": ("view_id",),
}

_SORTED_ID_LISTS = {
    "component_ids",
    "deform_region_ids",
    "dependency_ids",
    "material_region_ids",
    "object_ids",
    "primitive_ids",
    "protected_boundary_ids",
    "source_material_ids",
    "vertex_ids",
}


def _item_key(item: Any, keys: tuple[str, ...]) -> tuple[Any, ...]:
    if not isinstance(item, dict):
        return ("",)
    return tuple(item.get(key, "") for key in keys)


def canonicalize_document(document: dict[str, Any]) -> dict[str, Any]:
    result = deepcopy(document)
    for name, keys in _COLLECTION_KEYS.items():
        value = result.get(name)
        if isinstance(value, list):
            value.sort(key=lambda item: _item_key(item, keys))
    _normalize_nested(result)
    return result


def _normalize_nested(value: Any, parent_key: str | None = None) -> None:
    if isinstance(value, dict):
        for key, child in value.items():
            if key == "vertex_bindings" and isinstance(child, list):
                child.sort(
                    key=lambda item: (
                        item.get("source_object_id", ""),
                        item.get("source_primitive_id", ""),
                    )
                )
            elif key in _SORTED_ID_LISTS and isinstance(child, list):
                child.sort()
            _normalize_nested(child, key)
    elif isinstance(value, list):
        for child in value:
            _normalize_nested(child, parent_key)


def canonical_json_bytes(value: Any) -> bytes:
    return (_encode(value) + "\n").encode("utf-8")


def _encode(value: Any) -> str:
    if value is None:
        return "null"
    if value is True:
        return "true"
    if value is False:
        return "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float):
        if not math.isfinite(value):
            raise CanonicalJSONError("non-finite float cannot be serialized")
        if value == 0.0:
            value = 0.0
        return format(value, ".6f")
    if isinstance(value, str):
        return json.dumps(value, ensure_ascii=False, separators=(",", ":"))
    if isinstance(value, list):
        return "[" + ",".join(_encode(item) for item in value) + "]"
    if isinstance(value, dict):
        if not all(isinstance(key, str) for key in value):
            raise CanonicalJSONError("canonical object keys must be strings")
        parts = []
        for key in sorted(value):
            parts.append(f"{_encode(key)}:{_encode(value[key])}")
        return "{" + ",".join(parts) + "}"
    raise CanonicalJSONError(f"unsupported canonical value type: {type(value).__name__}")

