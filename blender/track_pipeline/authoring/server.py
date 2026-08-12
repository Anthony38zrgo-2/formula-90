"""Local Track Authoring server.

Serves the Vue 3 + SVG.js editor and a small JSON API backed by
:class:`TrackSession`. The SVG document is canonical authority; the server
sanitizes every save and reports validation without running Blender.

The served track is chosen explicitly via ``--source`` (a readable SVG path)
or ``--track`` (a track id resolved to its tip revision in ``--tracks-dir``);
without either, the test fixture is served so existing smoke tests keep
working.

Run:
    python authoring/server.py [--port 8400] [--source path/to/track.svg]
    python authoring/server.py --track fixture_minimal --tracks-dir tracks
Then open http://localhost:8400/
"""

from __future__ import annotations

import argparse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import json
import sys

# Ensure this package and the track_pipeline modules are importable regardless
# of the caller's working directory.
_PACKAGE = Path(__file__).resolve().parent
_PIPELINE = _PACKAGE.parent
for _path in (str(_PIPELINE), str(_PACKAGE)):
    if _path not in sys.path:
        sys.path.insert(0, _path)

from authoring_store import TrackSession, tip_revision_canonical_path
from asset_registry import AssetRegistry, load_registry, validate_registry
from svg_profile import TRACK_ID_RE, sha256_file

REPO = Path(__file__).resolve().parents[3]
STATIC = Path(__file__).resolve().parent / "static"
WORKSPACE_DIR = Path(__file__).resolve().parent / "workspace"
DEFAULT_SOURCE = REPO / "blender" / "track_pipeline" / "tests" / "fixtures" / "minimal_track.svg"
DEFAULT_REGISTRY = _PIPELINE / "configs" / "asset_registry.json"

CONTENT_TYPES = {
    ".html": "text/html; charset=utf-8",
    ".js": "text/javascript; charset=utf-8",
    ".css": "text/css; charset=utf-8",
    ".svg": "image/svg+xml; charset=utf-8",
    ".json": "application/json; charset=utf-8",
    ".png": "image/png",
}


def resolve_source(
    source_arg: str | None,
    track_arg: str | None,
    tracks_dir: Path | None,
    default_source: Path,
) -> Path:
    """Return the SVG source path to serve, validating --source/--track.

    ``--source`` must point at a readable file. ``--track`` resolves to the tip
    revision's canonical SVG and therefore requires ``--tracks-dir``. Supplying
    both is refused; without either the default (fixture) source is used.
    """
    if source_arg is not None and track_arg is not None:
        raise ValueError("use either --source or --track, not both")
    if source_arg is not None:
        path = Path(source_arg).resolve()
        if not path.is_file():
            raise ValueError(f"source not found: {source_arg}")
        try:
            with path.open("rb") as handle:
                handle.read(1)
        except OSError as exc:
            raise ValueError(f"source not readable: {source_arg} ({exc})") from exc
        return path
    if track_arg is not None:
        if tracks_dir is None:
            raise ValueError("--track requires --tracks-dir")
        if not TRACK_ID_RE.match(track_arg):
            raise ValueError(
                f"invalid track id {track_arg!r} "
                f"(must match {TRACK_ID_RE.pattern})"
            )
        canonical = tip_revision_canonical_path(tracks_dir, track_arg)
        if canonical is None:
            raise ValueError(
                f"no revision found for track {track_arg!r} under {tracks_dir}"
            )
        return canonical
    return Path(default_source).resolve()


def assets_payload(registry: AssetRegistry, registry_path: Path) -> dict:
    """Build the registry listing served by ``GET /api/assets``."""
    errors = validate_registry(registry)
    if errors:
        return {"ok": False, "diagnostics": errors}
    assets = [
        {
            "id": spec.id,
            "kind": spec.kind,
            "category": spec.category,
            "dimensions_m": dict(spec.dimensions_m),
            "preview": spec.preview,
            "budget": dict(spec.budget),
            "collision_class": spec.collision_class,
        }
        for spec in registry.specs
    ]
    return {
        "ok": True,
        "assets": assets,
        "count": len(assets),
        "registry_sha256": sha256_file(Path(registry_path)),
    }


class AuthoringHandler(BaseHTTPRequestHandler):
    session: TrackSession | None = None
    registry: AssetRegistry | None = None
    registry_path: Path | None = None

    def _json(self, payload: dict, status: int = 200) -> None:
        body = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _body(self) -> str:
        length = int(self.headers.get("Content-Length", "0"))
        return self.rfile.read(length).decode("utf-8")

    def _read_body_json(self) -> dict:
        try:
            return json.loads(self._body())
        except json.JSONDecodeError:
            return {}

    def _serve_static(self, path: str) -> None:
        rel = path.lstrip("/") or "index.html"
        target = (STATIC / rel).resolve()
        if not str(target).startswith(str(STATIC.resolve())) or not target.is_file():
            self.send_error(404, "not found")
            return
        body = target.read_bytes()
        self.send_response(200)
        self.send_header("Content-Type", CONTENT_TYPES.get(target.suffix, "application/octet-stream"))
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self) -> None:
        session = self.session
        if session is None:
            self.send_error(503, "session not initialized")
            return
        path = self.path.split("?")[0]
        if path == "/api/track":
            self._json(session.state())
        elif path == "/api/validate":
            self._json(session.validate())
        elif path == "/api/state":
            self._json(session.state())
        elif path == "/api/assets":
            registry = self.registry
            if registry is None or self.registry_path is None:
                self._json({"ok": False, "diagnostics": ["asset registry not initialized"]}, status=503)
                return
            self._json(assets_payload(registry, self.registry_path))
        else:
            self._serve_static(path)

    def do_POST(self) -> None:
        session = self.session
        if session is None:
            self.send_error(503, "session not initialized")
            return
        path = self.path.split("?")[0]
        if path == "/api/undo":
            self._json(session.undo_step())
        elif path == "/api/redo":
            self._json(session.redo_step())
        elif path == "/api/track":
            payload = self._read_body_json()
            try:
                self._json(session.save(payload.get("svg", "")), status=200)
            except Exception as exc:  # noqa: BLE001 - report actionable diagnostic
                self._json({"ok": False, "error": str(exc)}, status=400)
        elif path == "/api/state":
            self._json(session.state())
        elif path == "/api/import":
            payload = self._read_body_json()
            svg_text = payload.get("svg", "")
            if not isinstance(svg_text, str) or not svg_text.strip():
                self._json(
                    {"ok": False, "stage": "input",
                     "diagnostics": ["POST body must contain a non-empty 'svg' field"]},
                    status=400,
                )
                return
            result = session.import_svg(svg_text)
            status = 200 if result.get("ok") else 400
            self._json(result, status=status)
        elif path == "/api/region":
            payload = self._read_body_json()
            operation = payload.get("operation")
            region_id = payload.get("region_id")
            params = payload.get("params", {})
            if not operation or not region_id or not isinstance(params, dict):
                self._json(
                    {"ok": False, "stage": "input",
                     "diagnostics": ["region request needs 'operation', 'region_id' and 'params'"]},
                    status=400,
                )
                return
            result = session.apply_region_operation(operation, region_id, params)
            status = 200 if result.get("ok") else 400
            self._json(result, status=status)
        else:
            self.send_error(404, "unknown endpoint")

    def log_message(self, fmt: str, *args) -> None:
        print(f"[authoring] {self.address_string()} {fmt % args}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Formula90s local track authoring server.")
    parser.add_argument("--port", type=int, default=8400)
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--source", default=None,
                        help="SVG track source to serve (default: test fixture).")
    parser.add_argument("--track", default=None,
                        help="Track id whose tip revision is served (requires --tracks-dir).")
    parser.add_argument("--registry", default=str(DEFAULT_REGISTRY),
                        help="Asset Registry JSON (default: configs/asset_registry.json).")
    parser.add_argument("--tracks-dir", default=None,
                        help="Directory for immutable revisions (default: in-memory only).")
    parser.add_argument("--git-repo-root", default=None,
                        help="Repository root for scoped commits (requires --tracks-dir).")
    args = parser.parse_args()

    tracks_dir = Path(args.tracks_dir) if args.tracks_dir else None
    git_repo_root = Path(args.git_repo_root) if args.git_repo_root else None

    registry_path = Path(args.registry)
    if not registry_path.is_file():
        print(f"FAIL registry not found: {registry_path}", file=sys.stderr)
        return 2

    try:
        source = resolve_source(args.source, args.track, tracks_dir, DEFAULT_SOURCE)
    except ValueError as exc:
        print(f"FAIL {exc}", file=sys.stderr)
        return 2

    workspace = WORKSPACE_DIR / f"{source.stem}.source.svg"
    session = TrackSession(
        source, workspace, repo_root=REPO,
        tracks_dir=tracks_dir,
        git_repo_root=git_repo_root,
    )
    registry = load_registry(registry_path, repo_root=REPO)
    AuthoringHandler.session = session
    AuthoringHandler.registry = registry
    AuthoringHandler.registry_path = registry_path

    server = ThreadingHTTPServer((args.host, args.port), AuthoringHandler)
    print(f"F90 track authoring server: http://{args.host}:{args.port}/")
    print(f"  source : {source}")
    print(f"  workspace : {workspace}")
    print(f"  registry : {registry_path}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nstopping")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
