

"""Loopback-only HTTP API for Vehicle Studio onboarding."""
from __future__ import annotations

import argparse
from hashlib import sha256
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
from threading import RLock
from typing import Any
from urllib.parse import parse_qs, urlparse
from uuid import uuid4

from .build_ir import BuildIRError, build_ir_bytes, compile_build_ir
from .cad_views import compile_cad_views
from .canonical import canonical_json_bytes
from .dimension_profiles import dimension_profiles
from .glb_scan import GLBScanError, load_existing_inspector, scan_glb
from .initial_revision import compile_initial_revision
from .materializer import MaterializationError, materialize_build_ir
from .onboarding import OnboardingError, OnboardingSession
from .semantic_suggestions import suggest_semantics


WILLIAMS_94_ANCHORS = "blender/williams94_wheels_retextured/wheel_anchor_map.json"
WILLIAMS_94_PREVIEW = "blender/williams94_wheels_retextured/geometry/F1_94_chassis_geometry.glb"
WILLIAMS_94_TEXTURE_ROOT = "blender/williams94_wheels_retextured/textures/albedo"
WILLIAMS_94_SOURCES = (
    "blender/williams94_wheels_retextured/geometry/F1_94_chassis_core_geometry.glb",
    "blender/williams94_wheels_retextured/geometry/F1_94_front_wing_nose_geometry.glb",
    "blender/williams94_wheels_retextured/geometry/F1_94_rear_wing_geometry.glb",
    "blender/williams94_wheels_retextured/geometry/F1_94_wheel_front_geometry.glb",
    "blender/williams94_wheels_retextured/geometry/F1_94_wheel_rear_geometry.glb",
)


class APIError(ValueError):
    def __init__(self, code: str, message: str, status: int = 400):
        self.code = code
        self.status = status
        super().__init__(message)


class OnboardingController:
    def __init__(self, repo_root: Path):
        self.repo_root = Path(repo_root).resolve()
        self.sessions: dict[str, OnboardingSession] = {}
        self.revisions: dict[str, Any] = {}
        self.build_plans: dict[str, dict[str, Any]] = {}
        self.materializations: dict[str, dict[str, Any]] = {}
        self._lock = RLock()

    def presets(self) -> dict[str, Any]:
        available = all((self.repo_root / path).is_file() for path in WILLIAMS_94_SOURCES)
        return {
            "presets": [{
                "preset_id": "williams94",
                "label": "Williams 1994 — modular GLB",
                "available": available,
                "source_count": len(WILLIAMS_94_SOURCES),
            }]
        }

    def dimensional_profiles(self) -> dict[str, Any]:
        return {"profiles": dimension_profiles()}

    def scan_preset(self, preset_id: str) -> dict[str, Any]:
        if preset_id != "williams94":
            raise APIError("UNKNOWN_PRESET", "preset is not allowlisted")
        scans = [
            scan_glb(path, repo_root=self.repo_root, full=True)
            for path in WILLIAMS_94_SOURCES
        ]
        suggestions = self._merge_suggestions(scans)
        suggestions["suggestions"].extend(self._williams_auxiliary_suggestions())
        suggestions["suggestions"].sort(key=lambda item: (
            item["semantic_kind"], item["proposed_role"],
            item["source_path"], item["source_name"],
        ))
        for index, item in enumerate(suggestions["suggestions"], 1):
            item["suggestion_id"] = f"suggestion-{index:04d}"
        material_sources = self._williams_material_sources()
        aggregate = sha256(
            canonical_json_bytes([
                {"path": scan["source"]["path"], "sha256": scan["source"]["sha256"]}
                for scan in scans
            ] + [{"path": item["path"], "sha256": item["sha256"]}
                 for item in material_sources])
        ).hexdigest().upper()
        materialization_source = scan_glb(
            WILLIAMS_94_PREVIEW, repo_root=self.repo_root, full=False
        )["source"]
        source = {
            "path": "blender/williams94_wheels_retextured",
            "sha256": aggregate,
            "files": [scan["source"] for scan in scans],
            "material_sources": material_sources,
            "baseline_dimensions": self._williams_baseline_dimensions(),
            "materialization_source": materialization_source,
        }
        session = OnboardingSession(source, suggestions)
        session_id = uuid4().hex
        with self._lock:
            self.sessions[session_id] = session
        return {
            "session_id": session_id,
            "scan": {
                "source": source,
                "findings": [
                    {**finding, "source_path": scan["source"]["path"]}
                    for scan in scans for finding in scan["findings"]
                ],
            },
            "suggestions": suggestions["suggestions"],
            "onboarding": session.snapshot(),
        }

    def _williams_material_sources(self) -> list[dict[str, Any]]:
        root = self.repo_root / WILLIAMS_94_TEXTURE_ROOT
        results = []
        for path in sorted(root.glob("*.png"), key=lambda item: item.name):
            relative = path.relative_to(self.repo_root).as_posix()
            results.append({
                "path": relative,
                "sha256": sha256(path.read_bytes()).hexdigest().upper(),
                "size_bytes": path.stat().st_size,
                "material_slot": f"MAT_{path.stem}",
            })
        if not results:
            raise APIError("MATERIAL_SOURCES_MISSING", "Williams albedo sources do not exist", 404)
        return results

    def _williams_baseline_dimensions(self) -> dict[str, float]:
        data = json.loads((self.repo_root / WILLIAMS_94_ANCHORS).read_text(encoding="utf-8"))
        def tire_dimensions(section: dict[str, list[float]]) -> tuple[float, float]:
            radius = abs(float(section["DATUM_TIRE_CONTACT_PATCH"][1]))
            width = abs(float(section["DATUM_WHEEL_INBOARD_FACE"][0]) - float(section["DATUM_WHEEL_OUTBOARD_FACE"][0]))
            return radius, width
        front_radius, front_width = tire_dimensions(data["front"])
        rear_radius, rear_width = tire_dimensions(data["rear"])
        anchors = data["godot_required_existing_chassis_nodes"]
        front_center_y = (float(anchors["JNT_WHEEL_FL"][1]) + float(anchors["JNT_WHEEL_FR"][1])) / 2.0
        rear_center_y = (float(anchors["JNT_WHEEL_RL"][1]) + float(anchors["JNT_WHEEL_RR"][1])) / 2.0
        ground_y = ((front_center_y - front_radius) + (rear_center_y - rear_radius)) / 2.0
        return {
            "ground_y_m": ground_y,
            "front_tire_radius_m": front_radius,
            "front_tire_width_m": front_width,
            "rear_tire_radius_m": rear_radius,
            "rear_tire_width_m": rear_width,
        }

    def _williams_auxiliary_suggestions(self) -> list[dict[str, Any]]:
        anchor_path = self.repo_root / WILLIAMS_94_ANCHORS
        data = json.loads(anchor_path.read_text(encoding="utf-8"))
        anchors = data["godot_required_existing_chassis_nodes"]
        results: list[dict[str, Any]] = []
        for corner in ("FL", "FR", "RL", "RR"):
            node = f"JNT_WHEEL_{corner}"
            results.append({
                "confidence": "high",
                "evidence": [{
                    "code": "declared_wheel_anchor",
                    "translation_m": anchors[node],
                }],
                "proposed_role": f"wheel_{corner.lower()}_anchor",
                "requires_human_confirmation": True,
                "semantic_kind": "frame",
                "source_kind": "json_datum",
                "source_name": node,
                "source_path": WILLIAMS_94_ANCHORS,
            })
        for axle, left, right in (
            ("front", anchors["JNT_WHEEL_FL"], anchors["JNT_WHEEL_FR"]),
            ("rear", anchors["JNT_WHEEL_RL"], anchors["JNT_WHEEL_RR"]),
        ):
            midpoint = [(left[i] + right[i]) / 2.0 for i in range(3)]
            results.append({
                "confidence": "high",
                "evidence": [{
                    "code": "derived_axle_midpoint",
                    "translation_m": midpoint,
                }],
                "proposed_role": f"{axle}_axle_center",
                "requires_human_confirmation": True,
                "semantic_kind": "frame",
                "source_kind": "derived_datum",
                "source_name": f"{axle.upper()}_AXLE_CENTER",
                "source_path": WILLIAMS_94_ANCHORS,
            })
        inspector = load_existing_inspector(self.repo_root)
        for role, relative_path, node_name in (
            ("front_wing_mount", WILLIAMS_94_SOURCES[1], "JNT_FRONT_WING_MOUNT"),
            ("rear_wing_mount", WILLIAMS_94_SOURCES[2], "JNT_REAR_WING_MOUNT"),
        ):
            gltf, _ = inspector.read_glb(str(self.repo_root / relative_path))
            node = next(item for item in gltf.get("nodes", []) if item.get("name") == node_name)
            matrix = inspector.mat_from_trs(node)
            results.append({
                "confidence": "high",
                "evidence": [{
                    "code": "explicit_glb_frame",
                    "translation_m": [matrix[i][3] for i in range(3)],
                }],
                "proposed_role": role,
                "requires_human_confirmation": True,
                "semantic_kind": "frame",
                "source_kind": "node",
                "source_name": node_name,
                "source_path": relative_path,
            })
        for role in ("engine_cover", "sidepods"):
            results.append({
                "confidence": "medium",
                "evidence": [{"code": "accepted_embedded_chassis_region"}],
                "proposed_role": role,
                "requires_human_confirmation": True,
                "semantic_kind": "component",
                "source_kind": "embedded_region",
                "source_name": "GEO_CHASSIS",
                "source_path": WILLIAMS_94_SOURCES[0],
            })
        return results

    def apply_command(
        self, session_id: str, expected_revision: int, command: dict[str, Any]
    ) -> dict[str, Any]:
        with self._lock:
            session = self.sessions.get(session_id)
            if session is None:
                raise APIError("SESSION_NOT_FOUND", "onboarding session does not exist", 404)
            return {"onboarding": session.apply(expected_revision, command)}

    def compile_revision(self, session_id: str, project_id: str) -> dict[str, Any]:
        with self._lock:
            session = self.sessions.get(session_id)
            if session is None:
                raise APIError("SESSION_NOT_FOUND", "onboarding session does not exist", 404)
            document = compile_initial_revision(session.snapshot(), project_id=project_id)
            payload = document.canonical_bytes()
            self.revisions[session_id] = document
            return {
                "project_id": project_id,
                "revision": 0,
                "document_sha256": sha256(payload).hexdigest().upper(),
                "document": document.canonical_dict(),
                "diagnostics": [item.as_dict() for item in document.diagnostics()],
                "read_only": False,
            }

    def compile_build(self, session_id: str, parameter_values: dict[str, float], width_policies: dict[str, str] | None = None) -> dict[str, Any]:
        with self._lock:
            document = self.revisions.get(session_id)
            if document is None:
                raise APIError("REVISION_NOT_FOUND", "compile revision zero before BuildIR", 409)
            if not isinstance(parameter_values, dict):
                raise APIError("MALFORMED_PARAMETERS", "parameter_values must be an object")
            build_ir = compile_build_ir(document, parameter_values=parameter_values, width_policies=width_policies)
            payload = build_ir_bytes(build_ir)
            self.build_plans[session_id] = build_ir
            return {
                "build_ir": build_ir,
                "build_ir_sha256": sha256(payload).hexdigest().upper(),
                "materialized": False,
                "published": False,
            }

    def materialize_build(self, session_id: str) -> dict[str, Any]:
        with self._lock:
            build_ir = self.build_plans.get(session_id)
            if build_ir is None:
                raise APIError("BUILD_IR_NOT_FOUND", "compile BuildIR before materialization", 409)
            report = materialize_build_ir(
                build_ir,
                repo_root=self.repo_root,
                staging_root=self.repo_root / "tools" / "vehicle_studio" / "staging",
            )
            self.materializations[session_id] = report
            return {
                "build_ir_sha256": report["build_ir_sha256"],
                "materialized": True,
                "published": False,
                "report": report,
            }

    def compile_cad(self, session_id: str) -> dict[str, Any]:
        with self._lock:
            document = self.revisions.get(session_id)
            if document is None:
                raise APIError("REVISION_NOT_FOUND", "compile revision zero before CAD views", 409)
            views, manifest = compile_cad_views(document, repo_root=self.repo_root)
            return {
                "manifest": manifest,
                "views": {
                    name.removesuffix(".svg"): content.decode("utf-8")
                    for name, content in views.items()
                },
            }

    def preview_recipe(self) -> dict[str, Any]:
        if not (self.repo_root / WILLIAMS_94_PREVIEW).is_file():
            raise APIError("PREVIEW_SOURCE_MISSING", "preview GLB does not exist", 404)
        return {
            "source_url": "/api/assets/source?asset_id=williams94-full",
            "label": "Browser preview — not Blender-authoritative",
            "camera": {
                "position": [4.5, 2.4, 5.2],
                "target": [0.0, 0.25, 0.0],
                "field_of_view_deg": 35,
            },
        }

    def preview_asset(self, asset_id: str) -> Path:
        if asset_id != "williams94-full":
            raise APIError("ASSET_NOT_ALLOWED", "asset id is not allowlisted", 404)
        path = (self.repo_root / WILLIAMS_94_PREVIEW).resolve()
        try:
            path.relative_to(self.repo_root)
        except ValueError as exc:
            raise APIError("ASSET_OUTSIDE_ROOT", "asset escapes repository", 403) from exc
        if not path.is_file():
            raise APIError("ASSET_NOT_FOUND", "preview asset does not exist", 404)
        return path

    def preview_variant(self, session_id: str) -> Path:
        report = self.materializations.get(session_id)
        if report is None:
            raise APIError("VARIANT_NOT_MATERIALIZED", "materialize a variant before preview", 404)
        path = Path(report["output_glb"]).resolve()
        staging = (self.repo_root / "tools" / "vehicle_studio" / "staging").resolve()
        try:
            path.relative_to(staging)
        except ValueError as exc:
            raise APIError("VARIANT_OUTSIDE_STAGING", "variant escapes staging", 403) from exc
        if not path.is_file():
            raise APIError("VARIANT_NOT_FOUND", "staged variant GLB is missing", 404)
        return path

    @staticmethod
    def _merge_suggestions(scans: list[dict[str, Any]]) -> dict[str, Any]:
        merged: list[dict[str, Any]] = []
        seen: set[tuple[str, str, str, str]] = set()
        for scan in scans:
            source_path = scan["source"]["path"]
            for item in suggest_semantics(scan)["suggestions"]:
                key = (
                    item["semantic_kind"], item["proposed_role"],
                    source_path, item["source_name"],
                )
                if key in seen:
                    continue
                seen.add(key)
                candidate = dict(item)
                candidate["source_path"] = source_path
                merged.append(candidate)
        merged.sort(key=lambda item: (
            item["semantic_kind"], item["proposed_role"],
            item["source_path"], item["source_name"],
        ))
        for index, item in enumerate(merged, 1):
            item["suggestion_id"] = f"suggestion-{index:04d}"
        return {
            "schema_version": "vehicle-semantic-suggestions/v1",
            "authority": "suggestion_only",
            "suggestions": merged,
        }


def make_handler(controller: OnboardingController):
    class Handler(BaseHTTPRequestHandler):
        def do_GET(self) -> None:
            try:
                path = urlparse(self.path).path
                if path == "/api/health":
                    self._send(HTTPStatus.OK, {"status": "ok"})
                elif path == "/api/onboarding/presets":
                    self._send(HTTPStatus.OK, controller.presets())
                elif path == "/api/dimension-profiles":
                    self._send(HTTPStatus.OK, controller.dimensional_profiles())
                elif path == "/api/preview/recipe":
                    self._send(HTTPStatus.OK, controller.preview_recipe())
                elif path == "/api/assets/source":
                    query = parse_qs(urlparse(self.path).query)
                    self._send_file(controller.preview_asset(query.get("asset_id", [""])[0]))
                elif path == "/api/assets/variant":
                    query = parse_qs(urlparse(self.path).query)
                    self._send_file(controller.preview_variant(query.get("session_id", [""])[0]))
                else:
                    raise APIError("NOT_FOUND", "route not found", 404)
            except APIError as exc:
                self._send(exc.status, {"error": {"code": exc.code, "message": str(exc)}})

        def do_POST(self) -> None:
            try:
                payload = self._read_json()
                path = urlparse(self.path).path
                if path == "/api/onboarding/scan":
                    result = controller.scan_preset(payload.get("preset_id"))
                elif path == "/api/onboarding/command":
                    result = controller.apply_command(
                        payload.get("session_id"),
                        payload.get("expected_revision"),
                        payload.get("command"),
                    )
                elif path == "/api/onboarding/compile":
                    result = controller.compile_revision(
                        payload.get("session_id"),
                        payload.get("project_id"),
                    )
                elif path == "/api/cad/compile":
                    result = controller.compile_cad(payload.get("session_id"))
                elif path == "/api/build/compile":
                    result = controller.compile_build(
                        payload.get("session_id"),
                        payload.get("parameter_values"),
                        payload.get("width_policies"),
                    )
                elif path == "/api/build/materialize":
                    result = controller.materialize_build(payload.get("session_id"))
                else:
                    raise APIError("NOT_FOUND", "route not found", 404)
                self._send(HTTPStatus.OK, result)
            except (APIError, BuildIRError, MaterializationError, OnboardingError, GLBScanError) as exc:
                status = getattr(exc, "status", 400)
                self._send(status, {"error": {"code": getattr(exc, "code", type(exc).__name__), "message": str(exc)}})
            except (json.JSONDecodeError, TypeError, ValueError):
                self._send(HTTPStatus.BAD_REQUEST, {"error": {"code": "MALFORMED_PAYLOAD", "message": "request body is invalid"}})

        def _read_json(self) -> dict[str, Any]:
            length = int(self.headers.get("Content-Length", "0"))
            if length <= 0 or length > 1_000_000:
                raise APIError("MALFORMED_PAYLOAD", "request body size is invalid")
            value = json.loads(self.rfile.read(length).decode("utf-8"))
            if not isinstance(value, dict):
                raise APIError("MALFORMED_PAYLOAD", "request body must be an object")
            return value

        def _send_file(self, path: Path) -> None:
            size = path.stat().st_size
            self.send_response(HTTPStatus.OK)
            self.send_header("Content-Type", "model/gltf-binary")
            self.send_header("Content-Length", str(size))
            self.send_header("Cache-Control", "no-store")
            self.end_headers()
            with path.open("rb") as handle:
                while chunk := handle.read(1024 * 1024):
                    self.wfile.write(chunk)

        def _send(self, status: int, payload: dict[str, Any]) -> None:
            body = json.dumps(payload, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
            self.send_response(status)
            self.send_header("Content-Type", "application/json; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Cache-Control", "no-store")
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, format: str, *args: Any) -> None:
            return

    return Handler


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8765)
    args = parser.parse_args()
    if args.host not in {"127.0.0.1", "localhost"}:
        raise SystemExit("Vehicle Studio API v1 only binds to loopback")
    controller = OnboardingController(args.repo_root)
    server = ThreadingHTTPServer((args.host, args.port), make_handler(controller))
    print(f"Vehicle Studio API: http://{args.host}:{args.port}", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
