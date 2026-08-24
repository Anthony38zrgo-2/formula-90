"""In-process contract for the local Vehicle Studio project service."""
from __future__ import annotations

from copy import deepcopy
from dataclasses import dataclass
from hashlib import sha256
from pathlib import Path, PurePosixPath, PureWindowsPath
from threading import RLock
from typing import Any

from .canonical import canonical_json_bytes
from .domain import VehicleDocument


class ServiceError(ValueError):
    def __init__(self, code: str, message: str):
        self.code = code
        super().__init__(message)


@dataclass
class _Project:
    document: dict[str, Any]
    revision: int = 0


class LocalProjectService:
    """Owns project state and safe commands; it never executes client strings."""

    COMMANDS = frozenset({"set_parameter"})

    def __init__(self, project_root: Path):
        self.project_root = Path(project_root).resolve()
        self._projects: dict[str, _Project] = {}
        self._build_requests: list[dict[str, Any]] = []
        self._lock = RLock()

    def create_project(self, project_id: str, document: dict[str, Any]) -> dict[str, Any]:
        self._validate_project_id(project_id)
        canonical = VehicleDocument.from_dict(document).canonical_dict()
        self._assert_source_confined(canonical)
        with self._lock:
            if project_id in self._projects:
                raise ServiceError("PROJECT_EXISTS", f"project {project_id!r} already exists")
            self._projects[project_id] = _Project(canonical)
            return self.snapshot(project_id)

    def snapshot(self, project_id: str) -> dict[str, Any]:
        with self._lock:
            project = self._require_project(project_id)
            data = deepcopy(project.document)
            payload = canonical_json_bytes(data)
            return {
                "project_id": project_id,
                "revision": project.revision,
                "document_sha256": sha256(payload).hexdigest().upper(),
                "document": data,
                "diagnostics": [
                    item.as_dict()
                    for item in VehicleDocument.from_dict(data).diagnostics()
                ],
                "read_only": False,
            }

    def apply_command(
        self, project_id: str, expected_revision: int, command: dict[str, Any]
    ) -> dict[str, Any]:
        if not isinstance(command, dict):
            raise ServiceError("MALFORMED_COMMAND", "command must be an object")
        command_type = command.get("type")
        if command_type not in self.COMMANDS:
            raise ServiceError("COMMAND_NOT_ALLOWED", "unknown command type")
        with self._lock:
            project = self._require_project(project_id)
            if expected_revision != project.revision:
                raise ServiceError("REVISION_CONFLICT", "project revision changed")
            candidate = deepcopy(project.document)
            if command_type == "set_parameter":
                self._set_parameter(candidate, command)
            canonical = VehicleDocument.from_dict(candidate).canonical_dict()
            self._assert_source_confined(canonical)
            project.document = canonical
            project.revision += 1
            return self.snapshot(project_id)

    def request_build(self, project_id: str, expected_revision: int) -> dict[str, Any]:
        with self._lock:
            project = self._require_project(project_id)
            if expected_revision != project.revision:
                raise ServiceError("REVISION_CONFLICT", "project revision changed")
            request = {
                "build_request_id": f"build-{len(self._build_requests) + 1:04d}",
                "project_id": project_id,
                "revision": project.revision,
                "status": "queued",
            }
            self._build_requests.append(request)
            return deepcopy(request)

    @property
    def build_request_count(self) -> int:
        return len(self._build_requests)

    def _set_parameter(self, document: dict[str, Any], command: dict[str, Any]) -> None:
        parameter_id = command.get("parameter_id")
        value = command.get("absolute_value")
        if not isinstance(parameter_id, str) or not isinstance(value, (int, float)):
            raise ServiceError(
                "MALFORMED_COMMAND", "set_parameter requires parameter_id and numeric absolute_value"
            )
        for parameter in document.get("parameters", []):
            if parameter.get("parameter_id") == parameter_id:
                parameter["absolute_value"] = value
                return
        raise ServiceError("UNKNOWN_PARAMETER", f"unknown parameter {parameter_id!r}")

    def _assert_source_confined(self, document: dict[str, Any]) -> None:
        source_path = document["source"]["path"]
        target = (self.project_root / Path(source_path)).resolve()
        try:
            target.relative_to(self.project_root)
        except ValueError as exc:
            raise ServiceError("SOURCE_OUTSIDE_ROOT", "source escapes project root") from exc

    def _require_project(self, project_id: str) -> _Project:
        try:
            return self._projects[project_id]
        except KeyError as exc:
            raise ServiceError("PROJECT_NOT_FOUND", f"unknown project {project_id!r}") from exc

    @staticmethod
    def _validate_project_id(project_id: str) -> None:
        if (
            not isinstance(project_id, str)
            or not project_id
            or PureWindowsPath(project_id).is_absolute()
            or PureWindowsPath(project_id).drive
            or PurePosixPath(project_id).is_absolute()
            or any(part in {"", ".", ".."} for part in PurePosixPath(project_id).parts)
            or "/" in project_id
            or "\\" in project_id
        ):
            raise ServiceError("INVALID_PROJECT_ID", "project_id must be a single safe segment")
