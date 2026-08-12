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
from authoring.authoring_store import TrackSession
from authoring.server import (
    DEFAULT_REGISTRY,
    AuthoringHandler,
)

REGIONS_FIXTURE = ROOT / "blender" / "track_pipeline" / "tests" / "fixtures" / "regions_track.svg"
REGIONS_SVG = REGIONS_FIXTURE.read_text(encoding="utf-8")


def _rmtree(path: Path):
    if path.exists():
        for child in path.iterdir():
            if child.is_dir():
                _rmtree(child)
            else:
                child.unlink()
        path.rmdir()


def _session(tracks_dir: Path | None = None):
    tmp = Path(tempfile.mkdtemp(prefix="f90_regions_"))
    tracks = tmp / "tracks"
    if tracks_dir is not None:
        tracks = Path(tracks_dir)
    session = TrackSession(
        REGIONS_FIXTURE, tmp / "workspace" / "regions.source.svg", repo_root=ROOT,
        tracks_dir=tracks,
    )
    return session, tmp


class EditorRegionsHttpTests(unittest.TestCase):
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

    def _region(self, body):
        return json.loads(body)

    def _count_instances(self, svg_text):
        return svg_text.count('data-role="asset-instance"')

    def test_region_move_returns_ok_and_changes_canonical(self):
        status, _, body = self._request(
            "POST", "/api/region",
            {"operation": "move", "region_id": "bushes_000", "params": {"dx": 5.0, "dz": -3.0}},
        )
        self.assertEqual(status, 200)
        data = self._region(body)
        self.assertTrue(data["ok"])
        self.assertEqual(data["stage"], "applied")
        self.assertEqual(data["track_id"], "regions_test")
        self.assertIn("5.0000", data["current"])
        self.assertGreaterEqual(data["undo_depth"], 1)
        self.assertIn('data-region-id="bushes_000"', data["current"])

    def test_region_move_increments_revision_count(self):
        tracks_dir = self.tmp / "revs"
        session, tmp = _session(tracks_dir)
        AuthoringHandler.session = session
        before = session.state()["revision_count"]
        status, _, body = self._request(
            "POST", "/api/region",
            {"operation": "move", "region_id": "bushes_000", "params": {"dx": 2.0, "dz": 1.0}},
        )
        self.assertEqual(status, 200)
        data = self._region(body)
        self.assertTrue(data["ok"])
        self.assertEqual(data["revision_count"], before + 1)
        self.assertIsNotNone(data["current_revision_sha256"])

    def test_region_scale_returns_ok(self):
        status, _, body = self._request(
            "POST", "/api/region",
            {"operation": "scale", "region_id": "bushes_000", "params": {"factor": 2.0}},
        )
        self.assertEqual(status, 200)
        data = self._region(body)
        self.assertTrue(data["ok"])
        self.assertIn('data-scale="2.0000"', data["current"])
        self.assertIn("30.0000,-10.0000", data["current"])

    def test_region_extend_returns_ok_and_adds_instances(self):
        before = self._count_instances(self.session.current)
        status, _, body = self._request(
            "POST", "/api/region",
            {
                "operation": "extend", "region_id": "bushes_000",
                "params": {
                    "new_boundary": [[-10, -10], [30, -10], [30, 30], [-10, 30]],
                    "asset_pool": ["bush_v2_01", "bush_v2_02"],
                },
            },
        )
        self.assertEqual(status, 200)
        data = self._region(body)
        self.assertTrue(data["ok"])
        self.assertEqual(data["stage"], "applied")
        self.assertGreater(self._count_instances(data["current"]), before)
        self.assertIn('data-target-count="', data["current"])

    def test_region_get_endpoint_does_not_exist(self):
        status, _, body = self._request("GET", "/api/region")
        self.assertEqual(status, 404)

    def test_region_invalid_operation_returns_false_with_diagnostics(self):
        status, _, body = self._request(
            "POST", "/api/region",
            {"operation": "teleport", "region_id": "bushes_000", "params": {}},
        )
        self.assertEqual(status, 400)
        data = self._region(body)
        self.assertFalse(data["ok"])
        self.assertTrue(data["diagnostics"])

    def test_region_unknown_region_id_returns_false(self):
        status, _, body = self._request(
            "POST", "/api/region",
            {"operation": "move", "region_id": "ghost_000", "params": {"dx": 1.0, "dz": 0.0}},
        )
        self.assertEqual(status, 400)
        data = self._region(body)
        self.assertFalse(data["ok"])
        self.assertTrue(any("region not found" in d for d in data["diagnostics"]))


if __name__ == "__main__":
    unittest.main()
