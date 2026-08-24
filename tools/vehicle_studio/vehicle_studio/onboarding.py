"""Assisted, command-driven semantic onboarding state."""
from __future__ import annotations

from copy import deepcopy
from typing import Any

from .canonical import canonical_json_bytes


REQUIRED_COMPONENT_ROLES = frozenset({
    "chassis", "nose", "front_wing", "rear_wing",
    "engine_cover", "sidepods", "wheel_fl", "wheel_fr", "wheel_rl", "wheel_rr",
})
REQUIRED_FRAME_ROLES = frozenset({
    "vehicle_origin", "front_axle_center", "rear_axle_center",
    "wheel_fl_anchor", "wheel_fr_anchor", "wheel_rl_anchor", "wheel_rr_anchor",
    "front_wing_mount", "rear_wing_mount",
})
OPTIONAL_COMPONENT_ROLES = frozenset({"cockpit", "driver"})


class OnboardingError(ValueError):
    def __init__(self, code: str, message: str):
        self.code = code
        super().__init__(message)


class OnboardingSession:
    def __init__(self, source: dict[str, Any], suggestions: dict[str, Any]):
        self.source = deepcopy(source)
        self.suggestions = {
            item["suggestion_id"]: deepcopy(item)
            for item in suggestions.get("suggestions", [])
        }
        self.components: dict[str, dict[str, Any]] = {}
        self.frames: dict[str, dict[str, Any]] = {}
        self.axes: dict[str, str] = {}
        self.ground_y_m: float | None = None
        self.symmetry_plane_x_m: float | None = None
        self.optional_roles: dict[str, str] = {}
        self.revision = 0

    def apply(self, expected_revision: int, command: dict[str, Any]) -> dict[str, Any]:
        if expected_revision != self.revision:
            raise OnboardingError("REVISION_CONFLICT", "onboarding revision changed")
        handlers = {
            "accept_suggestion": self._accept_suggestion,
            "accept_suggestions": self._accept_suggestions,
            "map_role": self._map_role,
            "set_axes": self._set_axes,
            "set_ground": self._set_ground,
            "set_symmetry": self._set_symmetry,
            "set_optional_role": self._set_optional_role,
        }
        handler = handlers.get(command.get("type")) if isinstance(command, dict) else None
        if handler is None:
            raise OnboardingError("COMMAND_NOT_ALLOWED", "unknown onboarding command")
        handler(command)
        self.revision += 1
        return self.snapshot()

    def snapshot(self) -> dict[str, Any]:
        missing = self._missing_requirements()
        return {
            "schema_version": "vehicle-onboarding/v1",
            "revision": self.revision,
            "source": deepcopy(self.source),
            "axes": dict(sorted(self.axes.items())),
            "ground_y_m": self.ground_y_m,
            "symmetry_plane_x_m": self.symmetry_plane_x_m,
            "components": sorted(self.components.values(), key=lambda item: item["semantic_role"]),
            "frames": sorted(self.frames.values(), key=lambda item: item["semantic_role"]),
            "optional_roles": dict(sorted(self.optional_roles.items())),
            "missing_requirements": missing,
            "ready_to_compile": not missing,
        }

    def accepted_mapping_bytes(self) -> bytes:
        snapshot = self.snapshot()
        if not snapshot["ready_to_compile"]:
            raise OnboardingError(
                "ONBOARDING_INCOMPLETE",
                "required semantic roles must be resolved before compilation",
            )
        return canonical_json_bytes(snapshot)

    def _accept_suggestions(self, command: dict[str, Any]) -> None:
        suggestion_ids = command.get("suggestion_ids")
        if not isinstance(suggestion_ids, list) or not suggestion_ids:
            raise OnboardingError("INVALID_SUGGESTION_BATCH", "suggestion_ids must be a non-empty array")
        if not all(isinstance(item, str) and item in self.suggestions for item in suggestion_ids):
            raise OnboardingError("UNKNOWN_SUGGESTION", "batch contains an unknown suggestion")
        suggestions = [self.suggestions[item] for item in suggestion_ids]
        roles = [item["proposed_role"] for item in suggestions]
        if len(roles) != len(set(roles)):
            raise OnboardingError("AMBIGUOUS_SUGGESTION_BATCH", "batch must contain one suggestion per role")
        for suggestion_id in suggestion_ids:
            self._accept_suggestion({"suggestion_id": suggestion_id})

    def _accept_suggestion(self, command: dict[str, Any]) -> None:
        suggestion_id = command.get("suggestion_id")
        try:
            suggestion = self.suggestions[suggestion_id]
        except (KeyError, TypeError) as exc:
            raise OnboardingError("UNKNOWN_SUGGESTION", "suggestion does not exist") from exc
        self._store_mapping(
            suggestion["semantic_kind"],
            suggestion["proposed_role"],
            suggestion["source_kind"],
            suggestion["source_name"],
            {
                "suggestion_id": suggestion_id,
                "source_path": suggestion.get("source_path"),
                "evidence": suggestion["evidence"],
            },
        )

    def _map_role(self, command: dict[str, Any]) -> None:
        self._store_mapping(
            command.get("semantic_kind"),
            command.get("semantic_role"),
            command.get("source_kind"),
            command.get("source_name"),
            {"manual": True},
        )

    def _store_mapping(
        self, kind: Any, role: Any, source_kind: Any, source_name: Any, provenance: dict[str, Any]
    ) -> None:
        if kind not in {"component", "frame"}:
            raise OnboardingError("INVALID_MAPPING", "semantic_kind must be component or frame")
        if not all(isinstance(value, str) and value for value in (role, source_kind, source_name)):
            raise OnboardingError("INVALID_MAPPING", "role and source identity are required")
        target = self.components if kind == "component" else self.frames
        target[role] = {
            "semantic_role": role,
            "source_kind": source_kind,
            "source_name": source_name,
            "provenance": deepcopy(provenance),
        }
        if kind == "component" and role in OPTIONAL_COMPONENT_ROLES:
            self.optional_roles[role] = "mapped"

    def _set_axes(self, command: dict[str, Any]) -> None:
        axes = {key: command.get(key) for key in ("right_axis", "up_axis", "forward_axis")}
        if axes != {"right_axis": "+X", "up_axis": "+Y", "forward_axis": "-Z"}:
            raise OnboardingError("INVALID_AXES", "v1 requires +X right, +Y up and -Z forward")
        self.axes = axes

    def _set_ground(self, command: dict[str, Any]) -> None:
        value = command.get("ground_y_m")
        if not isinstance(value, (int, float)):
            raise OnboardingError("INVALID_GROUND", "ground_y_m must be numeric")
        self.ground_y_m = float(value)

    def _set_symmetry(self, command: dict[str, Any]) -> None:
        value = command.get("symmetry_plane_x_m")
        if not isinstance(value, (int, float)):
            raise OnboardingError("INVALID_SYMMETRY", "symmetry_plane_x_m must be numeric")
        self.symmetry_plane_x_m = float(value)

    def _set_optional_role(self, command: dict[str, Any]) -> None:
        role, status = command.get("semantic_role"), command.get("status")
        if role not in OPTIONAL_COMPONENT_ROLES or status not in {"mapped", "absent", "combined"}:
            raise OnboardingError("INVALID_OPTIONAL_ROLE", "optional role/status is invalid")
        self.optional_roles[role] = status

    def _missing_requirements(self) -> list[str]:
        missing = [
            *(f"component:{role}" for role in REQUIRED_COMPONENT_ROLES - self.components.keys()),
            *(f"frame:{role}" for role in REQUIRED_FRAME_ROLES - self.frames.keys()),
            *(f"optional:{role}" for role in OPTIONAL_COMPONENT_ROLES - self.optional_roles.keys()),
        ]
        if not self.axes:
            missing.append("coordinate_axes")
        if self.ground_y_m is None:
            missing.append("ground_plane")
        if self.symmetry_plane_x_m is None:
            missing.append("symmetry_plane")
        return sorted(missing)






