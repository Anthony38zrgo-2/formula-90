"""VehicleDocument v1 semantic validation and canonical serialization."""

from __future__ import annotations

from copy import deepcopy
import json
import math
from pathlib import Path, PurePosixPath, PureWindowsPath
import re
from typing import Any, Iterable

from .canonical import CanonicalJSONError, canonical_json_bytes, canonicalize_document
from .diagnostics import Diagnostic, Severity, sorted_diagnostics


ID_RE = re.compile(r"^[a-z][a-z0-9]*(?:[._-][a-z0-9]+)*$")
SHA256_RE = re.compile(r"^[0-9A-F]{64}$")
REQUIRED_FRAME_ROLES = {
    "vehicle_origin",
    "front_axle_center",
    "rear_axle_center",
    "wheel_fl",
    "wheel_fr",
    "wheel_rl",
    "wheel_rr",
    "front_wing_mount",
    "rear_wing_mount",
}
COLLECTION_ID_FIELDS = {
    "component_map": "component_id",
    "frames": "frame_id",
    "stations": "station_id",
    "deform_regions": "region_id",
    "parameters": "parameter_id",
    "material_regions": "region_id",
    "material_recipes": "recipe_id",
    "livery_layers": "layer_id",
    "view_definitions": "view_id",
}


class VehicleDocumentError(ValueError):
    def __init__(self, diagnostics: list[Diagnostic]):
        self.diagnostics = sorted_diagnostics(diagnostics)
        super().__init__(
            "; ".join(f"{item.code}: {item.message}" for item in self.diagnostics)
        )


class VehicleDocument:
    def __init__(self, data: dict[str, Any]):
        self.data = deepcopy(data)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "VehicleDocument":
        if not isinstance(data, dict):
            raise TypeError("VehicleDocument root must be an object")
        return cls(data)

    @classmethod
    def load(cls, path: Path) -> "VehicleDocument":
        with Path(path).open("r", encoding="utf-8") as handle:
            data = json.load(handle)
        return cls.from_dict(data)

    def diagnostics(self) -> list[Diagnostic]:
        diagnostics: list[Diagnostic] = []
        self._validate_root(diagnostics)
        self._validate_finite(self.data, "", diagnostics)
        self._validate_ids(diagnostics)
        self._validate_source(diagnostics)
        self._validate_coordinate_system(diagnostics)
        self._validate_frames(diagnostics)
        self._validate_references(diagnostics)
        self._validate_views(diagnostics)
        self._validate_parameters(diagnostics)
        self._validate_policy(diagnostics)
        return sorted_diagnostics(diagnostics)

    def is_valid(self) -> bool:
        return not any(item.severity >= Severity.ERROR for item in self.diagnostics())

    def canonical_dict(self) -> dict[str, Any]:
        diagnostics = self.diagnostics()
        if any(item.severity >= Severity.ERROR for item in diagnostics):
            raise VehicleDocumentError(diagnostics)
        return canonicalize_document(self.data)

    def canonical_bytes(self) -> bytes:
        try:
            return canonical_json_bytes(self.canonical_dict())
        except CanonicalJSONError as exc:
            diagnostic = Diagnostic(
                code="VEHICLE_DOCUMENT_CANONICAL_JSON",
                severity=Severity.FATAL,
                message=str(exc),
            )
            raise VehicleDocumentError([diagnostic]) from exc

    def _error(
        self,
        diagnostics: list[Diagnostic],
        code: str,
        message: str,
        *,
        path: str | None = None,
        object_id: str | None = None,
        metadata: dict[str, Any] | None = None,
    ) -> None:
        diagnostics.append(
            Diagnostic(
                code=code,
                severity=Severity.ERROR,
                message=message,
                path=path,
                object_id=object_id,
                metadata=metadata or {},
            )
        )

    def _validate_root(self, diagnostics: list[Diagnostic]) -> None:
        if self.data.get("schema_version") != 1:
            self._error(
                diagnostics,
                "VEHICLE_DOCUMENT_SCHEMA_VERSION",
                "schema_version must equal 1",
                path="/schema_version",
            )
        for field in COLLECTION_ID_FIELDS:
            if not isinstance(self.data.get(field), list):
                self._error(
                    diagnostics,
                    "VEHICLE_DOCUMENT_COLLECTION",
                    f"{field} must be an array",
                    path=f"/{field}",
                )

    def _validate_finite(
        self, value: Any, path: str, diagnostics: list[Diagnostic]
    ) -> None:
        if isinstance(value, float) and not math.isfinite(value):
            self._error(
                diagnostics,
                "VEHICLE_DOCUMENT_NON_FINITE",
                "numeric values must be finite",
                path=path or "/",
            )
        elif isinstance(value, dict):
            for key, child in value.items():
                self._validate_finite(child, f"{path}/{key}", diagnostics)
        elif isinstance(value, list):
            for index, child in enumerate(value):
                self._validate_finite(child, f"{path}/{index}", diagnostics)

    def _iter_entities(self) -> Iterable[tuple[str, int, str, Any]]:
        for collection, id_field in COLLECTION_ID_FIELDS.items():
            values = self.data.get(collection)
            if not isinstance(values, list):
                continue
            for index, item in enumerate(values):
                entity_id = item.get(id_field) if isinstance(item, dict) else None
                yield collection, index, id_field, entity_id

    def _validate_ids(self, diagnostics: list[Diagnostic]) -> None:
        seen: dict[str, str] = {}
        for collection, index, id_field, entity_id in self._iter_entities():
            path = f"/{collection}/{index}/{id_field}"
            if not isinstance(entity_id, str) or not ID_RE.fullmatch(entity_id):
                self._error(
                    diagnostics,
                    "VEHICLE_DOCUMENT_ID_FORMAT",
                    "persistent ID has invalid format",
                    path=path,
                )
                continue
            if entity_id in seen:
                self._error(
                    diagnostics,
                    "VEHICLE_DOCUMENT_DUPLICATE_ID",
                    f"duplicate persistent ID {entity_id!r}",
                    path=path,
                    object_id=entity_id,
                    metadata={"first_path": seen[entity_id]},
                )
            else:
                seen[entity_id] = path

    def _validate_source(self, diagnostics: list[Diagnostic]) -> None:
        source = self.data.get("source")
        if not isinstance(source, dict):
            self._error(
                diagnostics,
                "VEHICLE_DOCUMENT_SOURCE",
                "source must be an object",
                path="/source",
            )
            return
        path = source.get("path")
        if not isinstance(path, str) or not _safe_relative_path(path):
            self._error(
                diagnostics,
                "VEHICLE_DOCUMENT_SOURCE_PATH",
                "source path must be safe and relative",
                path="/source/path",
            )
        for field in ("sha256", "evaluated_geometry_sha256"):
            value = source.get(field)
            if not isinstance(value, str) or not SHA256_RE.fullmatch(value):
                self._error(
                    diagnostics,
                    "VEHICLE_DOCUMENT_SOURCE_HASH",
                    f"{field} must be uppercase SHA-256",
                    path=f"/source/{field}",
                )

    def _validate_coordinate_system(self, diagnostics: list[Diagnostic]) -> None:
        coords = self.data.get("coordinate_system")
        expected = {
            "unit": "meter",
            "right_axis": "+X",
            "up_axis": "+Y",
            "forward_axis": "-Z",
        }
        if not isinstance(coords, dict):
            self._error(
                diagnostics,
                "VEHICLE_DOCUMENT_AXIS_CONVENTION",
                "coordinate_system must be an object",
                path="/coordinate_system",
            )
            return
        for field, expected_value in expected.items():
            if coords.get(field) != expected_value:
                self._error(
                    diagnostics,
                    "VEHICLE_DOCUMENT_AXIS_CONVENTION",
                    f"{field} must equal {expected_value}",
                    path=f"/coordinate_system/{field}",
                )

    def _validate_frames(self, diagnostics: list[Diagnostic]) -> None:
        frames = self.data.get("frames")
        if not isinstance(frames, list):
            return
        by_id = {
            item.get("frame_id"): item
            for item in frames
            if isinstance(item, dict) and isinstance(item.get("frame_id"), str)
        }
        roles: dict[str, list[str]] = {}
        for index, frame in enumerate(frames):
            if not isinstance(frame, dict):
                continue
            frame_id = frame.get("frame_id")
            role = frame.get("semantic_role")
            if isinstance(role, str) and isinstance(frame_id, str):
                roles.setdefault(role, []).append(frame_id)
            parent = frame.get("parent_frame_id")
            if parent is not None and parent not in by_id:
                self._error(
                    diagnostics,
                    "VEHICLE_DOCUMENT_FRAME_REFERENCE",
                    f"unknown parent frame {parent!r}",
                    path=f"/frames/{index}/parent_frame_id",
                    object_id=frame_id if isinstance(frame_id, str) else None,
                )
        for role in sorted(REQUIRED_FRAME_ROLES):
            count = len(roles.get(role, []))
            if count != 1:
                self._error(
                    diagnostics,
                    "VEHICLE_DOCUMENT_REQUIRED_FRAME",
                    f"required frame role {role!r} must occur exactly once",
                    path="/frames",
                    metadata={"count": count, "role": role},
                )
        self._validate_frame_cycles(by_id, diagnostics)
        front = _frame_translation_for_role(frames, "front_axle_center")
        rear = _frame_translation_for_role(frames, "rear_axle_center")
        if front is not None and rear is not None and not front[2] < rear[2]:
            self._error(
                diagnostics,
                "VEHICLE_DOCUMENT_AXLE_ORDER",
                "front axle Z must be less than rear axle Z when forward is -Z",
                path="/frames",
            )

    def _validate_frame_cycles(
        self, by_id: dict[str, dict[str, Any]], diagnostics: list[Diagnostic]
    ) -> None:
        for frame_id in sorted(by_id):
            visited: set[str] = set()
            current: str | None = frame_id
            while current is not None and current in by_id:
                if current in visited:
                    self._error(
                        diagnostics,
                        "VEHICLE_DOCUMENT_FRAME_CYCLE",
                        f"frame hierarchy cycle includes {current!r}",
                        path="/frames",
                        object_id=frame_id,
                    )
                    break
                visited.add(current)
                parent = by_id[current].get("parent_frame_id")
                current = parent if isinstance(parent, str) else None

    def _validate_references(self, diagnostics: list[Diagnostic]) -> None:
        component_ids = _id_set(self.data.get("component_map"), "component_id")
        region_ids = _id_set(self.data.get("deform_regions"), "region_id")
        material_region_ids = _id_set(self.data.get("material_regions"), "region_id")
        parameter_ids = _id_set(self.data.get("parameters"), "parameter_id")
        for index, component in enumerate(self.data.get("component_map", [])):
            if not isinstance(component, dict):
                continue
            self._check_refs(
                diagnostics,
                component.get("deform_region_ids"),
                region_ids,
                "VEHICLE_DOCUMENT_REGION_REFERENCE",
                f"/component_map/{index}/deform_region_ids",
            )
            self._check_refs(
                diagnostics,
                component.get("material_region_ids"),
                material_region_ids,
                "VEHICLE_DOCUMENT_MATERIAL_REGION_REFERENCE",
                f"/component_map/{index}/material_region_ids",
            )
        for index, region in enumerate(self.data.get("material_regions", [])):
            if isinstance(region, dict):
                self._check_refs(
                    diagnostics,
                    region.get("component_ids"),
                    component_ids,
                    "VEHICLE_DOCUMENT_COMPONENT_REFERENCE",
                    f"/material_regions/{index}/component_ids",
                )
        for index, layer in enumerate(self.data.get("livery_layers", [])):
            if isinstance(layer, dict):
                ref = layer.get("material_region_id")
                if ref not in material_region_ids:
                    self._error(
                        diagnostics,
                        "VEHICLE_DOCUMENT_MATERIAL_REGION_REFERENCE",
                        f"unknown material region {ref!r}",
                        path=f"/livery_layers/{index}/material_region_id",
                    )
        for index, parameter in enumerate(self.data.get("parameters", [])):
            if isinstance(parameter, dict):
                self._check_refs(
                    diagnostics,
                    parameter.get("dependency_ids"),
                    parameter_ids,
                    "VEHICLE_DOCUMENT_PARAMETER_REFERENCE",
                    f"/parameters/{index}/dependency_ids",
                )

    def _check_refs(
        self,
        diagnostics: list[Diagnostic],
        values: Any,
        valid: set[str],
        code: str,
        path: str,
    ) -> None:
        if not isinstance(values, list):
            return
        for value in values:
            if value not in valid:
                self._error(
                    diagnostics,
                    code,
                    f"unknown reference {value!r}",
                    path=path,
                )

    def _validate_views(self, diagnostics: list[Diagnostic]) -> None:
        for index, view in enumerate(self.data.get("view_definitions", [])):
            if not isinstance(view, dict):
                continue
            axes = [
                view.get("horizontal_axis"),
                view.get("vertical_axis"),
                view.get("depth_axis"),
            ]
            bases = [axis[-1:] for axis in axes if isinstance(axis, str)]
            if len(bases) != 3 or len(set(bases)) != 3:
                self._error(
                    diagnostics,
                    "VEHICLE_DOCUMENT_VIEW_AXES",
                    "view axes must use X, Y and Z exactly once",
                    path=f"/view_definitions/{index}",
                    object_id=view.get("view_id"),
                )

    def _validate_parameters(self, diagnostics: list[Diagnostic]) -> None:
        for index, parameter in enumerate(self.data.get("parameters", [])):
            if not isinstance(parameter, dict):
                continue
            low = parameter.get("minimum")
            high = parameter.get("maximum")
            baseline = parameter.get("baseline_value")
            value = parameter.get("absolute_value")
            if all(isinstance(item, (int, float)) for item in (low, high)):
                if low > high:
                    self._error(
                        diagnostics,
                        "VEHICLE_DOCUMENT_PARAMETER_BOUNDS",
                        "parameter minimum exceeds maximum",
                        path=f"/parameters/{index}",
                        object_id=parameter.get("parameter_id"),
                    )
                for field, candidate in (
                    ("baseline_value", baseline),
                    ("absolute_value", value),
                ):
                    if isinstance(candidate, (int, float)) and not low <= candidate <= high:
                        self._error(
                            diagnostics,
                            "VEHICLE_DOCUMENT_PARAMETER_BOUNDS",
                            f"{field} is outside parameter bounds",
                            path=f"/parameters/{index}/{field}",
                            object_id=parameter.get("parameter_id"),
                        )

    def _validate_policy(self, diagnostics: list[Diagnostic]) -> None:
        policy = self.data.get("validation_policy")
        required = {
            "topology_changes_allowed": False,
            "ground_contact_required": True,
            "uv_preservation_required": True,
            "source_mutation_allowed": False,
        }
        if not isinstance(policy, dict):
            self._error(
                diagnostics,
                "VEHICLE_DOCUMENT_POLICY",
                "validation_policy must be an object",
                path="/validation_policy",
            )
            return
        for field, expected in required.items():
            if policy.get(field) is not expected:
                self._error(
                    diagnostics,
                    "VEHICLE_DOCUMENT_POLICY",
                    f"{field} must equal {expected}",
                    path=f"/validation_policy/{field}",
                )


def _safe_relative_path(value: str) -> bool:
    if not value or "\n" in value or "\r" in value:
        return False
    windows = PureWindowsPath(value)
    posix = PurePosixPath(value.replace("\\", "/"))
    if windows.is_absolute() or posix.is_absolute() or windows.drive:
        return False
    return ".." not in posix.parts


def _id_set(values: Any, field: str) -> set[str]:
    if not isinstance(values, list):
        return set()
    return {
        item[field]
        for item in values
        if isinstance(item, dict) and isinstance(item.get(field), str)
    }


def _frame_translation_for_role(
    frames: list[Any], role: str
) -> list[float] | None:
    matches = [
        frame
        for frame in frames
        if isinstance(frame, dict) and frame.get("semantic_role") == role
    ]
    if len(matches) != 1:
        return None
    transform = matches[0].get("transform")
    translation = transform.get("translation_m") if isinstance(transform, dict) else None
    if (
        isinstance(translation, list)
        and len(translation) == 3
        and all(isinstance(item, (int, float)) for item in translation)
    ):
        return translation
    return None

