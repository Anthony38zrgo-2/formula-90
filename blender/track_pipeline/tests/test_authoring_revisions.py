import unittest
from pathlib import Path
import hashlib
import json
import os
import stat
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "blender" / "track_pipeline"))

from authoring.authoring_store import (
    CANONICAL_FILENAME,
    MANIFEST_FILENAME,
    RevisionError,
    TrackSession,
)
from svg_sanitizer import canonical_xml, sanitize_svg

SOURCE = ROOT / "blender" / "track_pipeline" / "tests" / "fixtures" / "minimal_track.svg"
BASE = SOURCE.read_text(encoding="utf-8")


def _edit(text: str, degrees: str) -> str:
    return text.replace('data-degrees="3.0"', f'data-degrees="{degrees}"')


def _git(repo: Path, *args):
    return subprocess.run(
        ["git", "-C", str(repo), *args],
        capture_output=True,
        text=True,
    )


def _commit_files(repo: Path, rev: str = "HEAD") -> set[str]:
    result = _git(repo, "show", "--format=", "--name-only", rev)
    return {line for line in result.stdout.splitlines() if line.strip()}


def _commit_count(repo: Path) -> int:
    result = _git(repo, "rev-list", "--count", "HEAD")
    return int(result.stdout.strip())


class TrackRevisionsTests(unittest.TestCase):
    def _session(self):
        tmp = Path(tempfile.mkdtemp(prefix="f90_revisions_"))
        self.addCleanup(_rmtree, tmp)
        repo = tmp / "repo"
        repo.mkdir(parents=True, exist_ok=True)
        init = _git(repo, "init", "-q")
        self.assertEqual(init.returncode, 0, init.stderr)
        _git(repo, "config", "user.name", "test")
        _git(repo, "config", "user.email", "test@example.com")
        session = TrackSession(
            SOURCE,
            tmp / "workspace" / "track.source.svg",
            repo_root=tmp,
            tracks_dir=repo / "tracks",
            git_repo_root=repo,
        )
        return session, tmp, repo

    def _revision_dirs(self, repo: Path) -> dict[str, Path]:
        revisions = repo / "tracks" / "fixture_minimal" / "revisions"
        if not revisions.is_dir():
            return {}
        return {entry.name: entry for entry in revisions.iterdir() if entry.is_dir()}

    # -- required: successful save -----------------------------------------

    def test_successful_save_creates_revision_and_scoped_commit(self):
        session, _, repo = self._session()
        state = session.save(_edit(BASE, "2.0"))
        sha = state["current_revision_sha256"]
        self.assertIsNotNone(sha)
        self.assertEqual(state["revision_count"], 1)

        revisions = self._revision_dirs(repo)
        self.assertEqual(list(revisions), [sha])
        revision_dir = revisions[sha]
        self.assertEqual(
            {p.name for p in revision_dir.iterdir()},
            {CANONICAL_FILENAME, MANIFEST_FILENAME},
        )

        canonical_bytes = (revision_dir / CANONICAL_FILENAME).read_bytes()
        manifest = json.loads((revision_dir / MANIFEST_FILENAME).read_text(encoding="utf-8"))
        self.assertEqual(manifest["revision_sha256"], sha)
        self.assertEqual(manifest["parent_revision_sha256"], None)
        self.assertEqual(
            manifest["artifacts"]["canonical"]["sha256"],
            hashlib.sha256(canonical_bytes).hexdigest(),
        )
        self.assertEqual(
            canonical_bytes,
            canonical_xml(sanitize_svg(_edit(BASE, "2.0").encode("utf-8"))),
        )

        self.assertEqual(_commit_count(repo), 1)
        expected = {
            f"tracks/fixture_minimal/revisions/{sha}/{CANONICAL_FILENAME}",
            f"tracks/fixture_minimal/revisions/{sha}/{MANIFEST_FILENAME}",
        }
        self.assertEqual(_commit_files(repo), expected)
        self.assertEqual(_git(repo, "status", "--porcelain").stdout.strip(), "")

    # -- required: empty-save idempotence ----------------------------------

    def test_empty_save_is_idempotent(self):
        session, _, repo = self._session()
        text = _edit(BASE, "2.0")
        session.save(text)
        session.save(text)

        self.assertEqual(len(self._revision_dirs(repo)), 1)
        self.assertEqual(_commit_count(repo), 1)

    def test_revision_sha_is_content_derived(self):
        first, _, _ = self._session()
        second, _, _ = self._session()
        sha_first = first.save(_edit(BASE, "2.0"))["current_revision_sha256"]
        sha_second = second.save(_edit(BASE, "2.0"))["current_revision_sha256"]
        self.assertEqual(sha_first, sha_second)

    # -- required: unrelated dirty-file preservation ------------------------

    def test_unrelated_dirty_file_preserved(self):
        session, _, repo = self._session()
        unrelated = repo / "unrelated.txt"
        unrelated.write_text("version 1\n", encoding="utf-8")
        _git(repo, "add", "unrelated.txt")
        _git(repo, "commit", "-q", "-m", "setup unrelated")
        unrelated.write_text("version 2 dirty\n", encoding="utf-8")

        session.save(_edit(BASE, "2.0"))

        self.assertEqual(unrelated.read_text(encoding="utf-8"), "version 2 dirty\n")
        porcelain = _git(repo, "status", "--porcelain").stdout
        self.assertIn(" M unrelated.txt", porcelain)
        self.assertNotIn("unrelated.txt", _commit_files(repo))
        self.assertEqual(_commit_count(repo), 2)
        self.assertEqual(_git(repo, "diff", "--cached", "--name-only").stdout.strip(), "")

    # -- required: additive revert ------------------------------------------

    def test_revert_is_additive_child_revision(self):
        session, _, repo = self._session()
        sha_first = session.save(_edit(BASE, "2.0"))["current_revision_sha256"]
        sha_second = session.save(_edit(BASE, "1.0"))["current_revision_sha256"]
        before_revert = _commit_count(repo)
        old_canonical_bytes = (self._revision_dirs(repo)[sha_first] / CANONICAL_FILENAME).read_bytes()

        state = session.revert(sha_first)

        revisions = self._revision_dirs(repo)
        self.assertEqual(len(revisions), 3)
        self.assertEqual(_commit_count(repo), before_revert + 1)
        self.assertNotIn("unrelated.txt", _commit_files(repo))

        old_revision = revisions[sha_first]
        self.assertEqual((old_revision / CANONICAL_FILENAME).read_bytes(), old_canonical_bytes)
        new_sha = state["current_revision_sha256"]
        self.assertNotEqual(new_sha, sha_first)
        new_revision = revisions[new_sha]
        new_manifest = json.loads(
            (new_revision / MANIFEST_FILENAME).read_text(encoding="utf-8")
        )
        self.assertEqual(new_manifest["parent_revision_sha256"], sha_second)
        self.assertEqual(new_manifest["source_revision_sha256"], sha_first)
        self.assertEqual(
            (new_revision / CANONICAL_FILENAME).read_bytes(),
            canonical_xml(sanitize_svg((old_revision / CANONICAL_FILENAME).read_bytes())),
        )
        self.assertEqual(
            session.current,
            (new_revision / CANONICAL_FILENAME).read_text(encoding="utf-8"),
        )

    # -- revision API guards ------------------------------------------------

    def test_revert_rejects_unknown_or_malformed_revision(self):
        session, _, repo = self._session()
        session.save(_edit(BASE, "2.0"))
        with self.assertRaises(RevisionError):
            session.revert("f" * 64)
        with self.assertRaises(RevisionError):
            session.revert("not-a-sha")

    def test_revert_requires_tracks_dir(self):
        tmp = Path(tempfile.mkdtemp(prefix="f90_revisions_legacy_"))
        self.addCleanup(_rmtree, tmp)
        session = TrackSession(SOURCE, tmp / "track.source.svg", repo_root=tmp)
        with self.assertRaises(RevisionError):
            session.revert("f" * 64)

    # -- point 1: source_sha256 is populated --------------------------------

    def test_state_source_sha256_is_source_file_hash(self):
        session, _, _ = self._session()
        expected = hashlib.sha256(SOURCE.read_bytes()).hexdigest()
        self.assertEqual(session.state()["source_sha256"], expected)

    # -- point 4: orphaned revision rolled back on commit failure -----------

    def test_commit_failure_rolls_back_written_revision(self):
        session, _, repo = self._session()
        session.save(_edit(BASE, "2.0"))  # commits fine (no hook yet)
        self.assertEqual(len(self._revision_dirs(repo)), 1)
        before = session.current

        hooks = repo / ".git" / "hooks"
        hooks.mkdir(parents=True, exist_ok=True)
        hook = hooks / "pre-commit"
        hook.write_text("#!/bin/sh\nexit 1\n", encoding="utf-8")
        os.chmod(hook, os.stat(hook).st_mode | stat.S_IEXEC)

        with self.assertRaises(Exception):
            session.save(_edit(BASE, "3.0"))

        self.assertEqual(len(self._revision_dirs(repo)), 1)
        self.assertEqual(_commit_count(repo), 1)
        self.assertEqual(session.current, before)

    # -- point 3: multiple heads refused -------------------------------------

    def test_multiple_revision_heads_refused(self):
        session, _, repo = self._session()
        session.save(_edit(BASE, "2.0"))

        shas = sorted(self._revision_dirs(repo))
        self.assertEqual(len(shas), 1)
        tip = shas[0]
        manifest_path = self._revision_dirs(repo)[tip] / MANIFEST_FILENAME
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        orphan_dir = repo / "tracks" / "fixture_minimal" / "revisions" / ("0" * 63 + "1")
        orphan_dir.mkdir(parents=True, exist_ok=True)
        orphan = dict(manifest)
        orphan["revision_sha256"] = orphan_dir.name
        orphan["parent_revision_sha256"] = None
        orphan["source_revision_sha256"] = None
        (orphan_dir / CANONICAL_FILENAME).write_text(
            session.current, encoding="utf-8"
        )
        (orphan_dir / MANIFEST_FILENAME).write_text(
            json.dumps(orphan), encoding="utf-8"
        )

        with self.assertRaises(RevisionError):
            session.state()


def _rmtree(path: Path):
    if path.exists():
        for child in path.iterdir():
            if child.is_dir():
                _rmtree(child)
            else:
                os.chmod(child, stat.S_IWRITE)
                child.unlink()
        path.rmdir()


if __name__ == "__main__":
    unittest.main()
