import unittest
from pathlib import Path
import json
import sys
import tempfile
import threading
from http.server import ThreadingHTTPServer
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "blender" / "track_pipeline"))

from asset_registry import load_registry
from authoring.authoring_store import TrackSession, tip_revision_canonical_path
from authoring.server import (
    DEFAULT_REGISTRY,
    DEFAULT_SOURCE,
    AuthoringHandler,
    assets_payload,
    resolve_source,
)

FIXTURE = ROOT / "blender" / "track_pipeline" / "tests" / "fixtures" / "minimal_track.svg"

VALID_SVG = FIXTURE.read_text(encoding="utf-8")


def _rmtree(path: Path):
    if path.exists():
        for child in path.iterdir():
            if child.is_dir():
                _rmtree(child)
            else:
                child.unlink()
        path.rmdir()


def _session(tracks_dir=None):
    tmp = Path(tempfile.mkdtemp(prefix="f90_server_"))
    tracks = tmp / "tracks"
    if tracks_dir is not None:
        tracks = Path(tracks_dir)
    session = TrackSession(
        FIXTURE, tmp / "workspace" / "track.source.svg", repo_root=ROOT,
        tracks_dir=tracks,
    )
    return session, tmp


class ResolveSourceTests(unittest.TestCase):
    def test_default_is_fixture(self):
        self.assertEqual(
            resolve_source(None, None, None, DEFAULT_SOURCE).resolve(),
            DEFAULT_SOURCE.resolve(),
        )

    def test_source_must_exist(self):
        with self.assertRaises(ValueError) as ctx:
            resolve_source(str(ROOT / "does_not_exist.svg"), None, None, DEFAULT_SOURCE)
        self.assertIn("not found", str(ctx.exception))

    def test_source_and_track_are_mutually_exclusive(self):
        with self.assertRaises(ValueError) as ctx:
            resolve_source(str(FIXTURE), "fixture_minimal", None, DEFAULT_SOURCE)
        self.assertIn("not both", str(ctx.exception))

    def test_track_requires_tracks_dir(self):
        with self.assertRaises(ValueError) as ctx:
            resolve_source(None, "fixture_minimal", None, DEFAULT_SOURCE)
        self.assertIn("--tracks-dir", str(ctx.exception))

    def test_track_rejects_invalid_id(self):
        with self.assertRaises(ValueError):
            resolve_source(None, "../evil", Path(tempfile.mkdtemp()), DEFAULT_SOURCE)

    def test_track_resolves_tip_revision(self):
        session, tmp = _session()
        session.save(VALID_SVG.replace('data-degrees="3.0"', 'data-degrees="2.0"'))
        tracks_dir = tmp / "tracks"
        canonical = tip_revision_canonical_path(tracks_dir, "fixture_minimal")
        self.assertIsNotNone(canonical)
        resolved = resolve_source(None, "fixture_minimal", tracks_dir, DEFAULT_SOURCE)
        self.assertEqual(resolved, canonical)

    def test_track_without_revision_rejected(self):
        tracks_dir = Path(tempfile.mkdtemp(prefix="f90_no_rev_"))
        with self.assertRaises(ValueError) as ctx:
            resolve_source(None, "ghost_track", tracks_dir, DEFAULT_SOURCE)
        self.assertIn("no revision", str(ctx.exception))


class ImportSvgTests(unittest.TestCase):
    def test_valid_import_returns_canonical_without_writing(self):
        session, tmp = _session()
        before = session.workspace.read_bytes()
        current_before = session.current
        result = TrackSession.import_svg(VALID_SVG)
        self.assertTrue(result["ok"])
        self.assertEqual(result["stage"], "imported")
        self.assertIn("canonical", result)
        self.assertIn("data-track-id=\"fixture_minimal\"", result["canonical"])
        self.assertTrue(result["validation"]["ok"])
        self.assertEqual(session.workspace.read_bytes(), before)
        self.assertEqual(session.current, current_before)

    def test_unsafe_import_rejected_with_diagnostics(self):
        malicious = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"><script>alert(1)</script></svg>'
        result = TrackSession.import_svg(malicious)
        self.assertFalse(result["ok"])
        self.assertEqual(result["stage"], "sanitize")
        self.assertTrue(result["diagnostics"])

    def test_unsupported_element_rejected(self):
        unsupported = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"><image href="x"/></svg>'
        result = TrackSession.import_svg(unsupported)
        self.assertFalse(result["ok"])
        self.assertTrue(any("image" in d.lower() for d in result["diagnostics"]))

    def test_invalid_geometry_still_reported_not_dropped(self):
        open_centerline = VALID_SVG.replace("Z", "")
        result = TrackSession.import_svg(open_centerline)
        self.assertTrue(result["ok"])
        self.assertEqual(result["stage"], "imported")
        self.assertFalse(result["validation"]["ok"])
        self.assertEqual(result["validation"]["stage"], "normalize")
        self.assertTrue(result["validation"]["diagnostics"])


class AssetsPayloadTests(unittest.TestCase):
    def test_payload_lists_all_registry_assets(self):
        registry = load_registry(DEFAULT_REGISTRY, repo_root=ROOT)
        payload = assets_payload(registry, DEFAULT_REGISTRY)
        self.assertTrue(payload["ok"])
        self.assertEqual(payload["count"], 20)
        self.assertEqual(len(payload["registry_sha256"]), 64)
        for asset in payload["assets"]:
            for key in ("id", "kind", "category", "dimensions_m", "preview", "budget", "collision_class"):
                self.assertIn(key, asset)
        ids = {a["id"] for a in payload["assets"]}
        self.assertIn("tree_v2_01", ids)
        self.assertIn("track_flag", ids)


class AuthoringHttpTests(unittest.TestCase):
    def setUp(self):
        self.session, self.tmp = _session()
        AuthoringHandler.session = self.session
        AuthoringHandler.registry = load_registry(DEFAULT_REGISTRY, repo_root=ROOT)
        AuthoringHandler.registry_path = DEFAULT_REGISTRY
        self.server = ThreadingHTTPServer(("127.0.0.1", 0), AuthoringHandler)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.addCleanup(self.server.shutdown)
        self.base = f"http://127.0.0.1:{self.server.server_address[1]}"

    def _request(self, method, path, body=None):
        data = json.dumps(body).encode("utf-8") if body is not None else None
        req = urllib.request.Request(
            self.base + path, data=data, method=method,
            headers={"Content-Type": "application/json"},
        )
        try:
            with urllib.request.urlopen(req, timeout=10) as resp:
                return resp.status, dict(resp.headers), resp.read()
        except urllib.error.HTTPError as exc:
            return exc.code, dict(exc.headers), exc.read()

    def test_validate_endpoint_ok(self):
        status, _, body = self._request("GET", "/api/validate")
        self.assertEqual(status, 200)
        data = json.loads(body)
        self.assertTrue(data["ok"])
        self.assertEqual(data["stage"], "validated")

    def test_track_and_state_endpoints(self):
        status, _, body = self._request("GET", "/api/track")
        self.assertEqual(status, 200)
        data = json.loads(body)
        self.assertEqual(data["track_id"], "fixture_minimal")
        self.assertEqual(len(data["source_sha256"]), 64)
        status, _, _ = self._request("GET", "/api/state")
        self.assertEqual(status, 200)

    def test_vendor_javascript_served_locally(self):
        status, headers, body = self._request("GET", "/vendor/svg.min.js")
        self.assertEqual(status, 200)
        self.assertIn("text/javascript", headers.get("Content-Type", ""))
        self.assertGreater(len(body), 10000)
        status, _, body = self._request("GET", "/vendor/vue.global.js")
        self.assertEqual(status, 200)
        self.assertIn("text/javascript", headers.get("Content-Type", ""))
        self.assertGreater(len(body), 10000)

    def test_assets_endpoint(self):
        status, _, body = self._request("GET", "/api/assets")
        self.assertEqual(status, 200)
        data = json.loads(body)
        self.assertTrue(data["ok"])
        self.assertEqual(data["count"], 20)

    def test_import_valid_returns_200(self):
        status, _, body = self._request("POST", "/api/import", {"svg": VALID_SVG})
        self.assertEqual(status, 200)
        data = json.loads(body)
        self.assertTrue(data["ok"])
        self.assertIn("canonical", data)

    def test_import_unsafe_returns_400(self):
        malicious = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"><script>x</script></svg>'
        status, _, body = self._request("POST", "/api/import", {"svg": malicious})
        self.assertEqual(status, 400)
        data = json.loads(body)
        self.assertFalse(data["ok"])
        self.assertTrue(data["diagnostics"])

    def test_import_missing_svg_field_returns_400(self):
        status, _, body = self._request("POST", "/api/import", {})
        self.assertEqual(status, 400)
        self.assertIn("svg", json.loads(body)["diagnostics"][0])

    def test_save_and_undo_roundtrip(self):
        modified = VALID_SVG.replace('data-track-id="fixture_minimal"', 'data-track-id="edited_via_http"')
        status, _, body = self._request("POST", "/api/track", {"svg": modified})
        self.assertEqual(status, 200)
        data = json.loads(body)
        self.assertEqual(data["undo_depth"], 1)
        self.assertIn("edited_via_http", data["current"])
        status, _, body = self._request("POST", "/api/undo")
        data = json.loads(body)
        self.assertEqual(data["track_id"], "fixture_minimal")


if __name__ == "__main__":
    unittest.main()
