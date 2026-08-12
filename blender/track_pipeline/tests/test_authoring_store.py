import unittest
from pathlib import Path
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "blender" / "track_pipeline"))

from svg_sanitizer import SVGSanitizeError
from authoring.authoring_store import TrackSession

SOURCE = ROOT / "blender" / "track_pipeline" / "tests" / "fixtures" / "minimal_track.svg"


class TrackSessionTests(unittest.TestCase):
    def _session(self):
        tmp = Path(tempfile.mkdtemp(prefix="f90_authoring_"))
        self.addCleanup(_rmtree, tmp)
        return TrackSession(SOURCE, tmp / "track.source.svg", repo_root=ROOT), tmp

    def test_initial_workspace_created_from_source(self):
        session, tmp = self._session()
        self.assertTrue(session.workspace.exists())
        self.assertEqual(session.undo, [])
        self.assertEqual(session.redo, [])

    def test_save_rejects_unsafe_svg(self):
        session, _ = self._session()
        with self.assertRaises(SVGSanitizeError):
            session.save('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"><script>x</script></svg>')

    def test_save_persists_canonical_and_undo_redo_roundtrip(self):
        session, _ = self._session()
        original = session.current
        modified = original.replace('data-track-id="fixture_minimal"', 'data-track-id="edited_track"')

        state = session.save(modified)
        self.assertEqual(state["undo_depth"], 1)
        self.assertNotEqual(session.current, original)
        self.assertIn("edited_track", session.workspace.read_text(encoding="utf-8"))

        session.undo_step()
        self.assertEqual(session.current, original)
        session.redo_step()
        self.assertNotEqual(session.current, original)

    def test_redo_cleared_on_new_save(self):
        session, _ = self._session()

        def retagged(tag):
            return session.current.replace('data-track-id="fixture_minimal"', f'data-track-id="{tag}"')

        session.save(retagged("rev_a"))
        session.save(retagged("rev_b"))
        session.undo_step()
        self.assertEqual(session.redo_depth, 1)
        session.save(retagged("rev_c"))
        self.assertEqual(session.redo_depth, 0)

    def test_validate_reports_collision_invariants(self):
        session, _ = self._session()
        result = session.validate()
        self.assertTrue(result["ok"])
        self.assertEqual(result["stage"], "validated")
        self.assertTrue(all(result["invariants"].values()))
        self.assertIn("centerline_length_m", result)
        self.assertGreater(result["asset_count"], 0)

    def test_validate_on_corrupt_document_returns_invalid(self):
        session, _ = self._session()
        session.current = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"></svg>'
        result = session.validate()
        self.assertFalse(result["ok"])
        self.assertIn("stage", result)

    def test_validate_on_unsafe_document_returns_sanitize_stage(self):
        session, _ = self._session()
        session.current = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"><use href="x"/></svg>'
        result = session.validate()
        self.assertFalse(result["ok"])
        self.assertEqual(result["stage"], "sanitize")


def _rmtree(path: Path):
    if path.exists():
        for child in path.iterdir():
            if child.is_dir():
                _rmtree(child)
            else:
                child.unlink()
        path.rmdir()


if __name__ == "__main__":
    unittest.main()
