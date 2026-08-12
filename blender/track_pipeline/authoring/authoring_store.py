"""Track authoring session store (testable, no HTTP).

Holds the current canonical F90 Track SVG plus in-memory undo/redo stacks.
Every save sanitizes the payload into the restricted profile before persisting,
so invalid or unsafe SVG can never become the active document. Normalization
reports collision invariants without invoking Blender.

When a ``tracks_dir`` is configured, every content-changing save also writes an
immutable revision at ``tracks/<track_id>/revisions/<revision_sha>/`` (canonical
SVG plus a provenance manifest referencing its parent revision) and, when a
``git_repo_root`` is configured, creates one scoped Git commit containing only
that track's revision files. Reverting to an older revision is additive: it
creates a new child revision from the old content and never runs reset/checkout.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any
from xml.etree import ElementTree as ET
import hashlib
import json
import re
import time

try:
    from git_service import GitScopedService
except ModuleNotFoundError:  # imported as part of the authoring package
    from .git_service import GitScopedService
from svg_profile import TRACK_ID_RE, sha256_bytes
from svg_sanitizer import SVGSanitizeError, canonical_xml, sanitize_svg
from svg_normalizer import NormalizeError, normalize
from vegetation_region_ops import RegionOperationError, apply_region_operation

REVISION_SCHEMA_VERSION = 1
CANONICAL_FILENAME = "track.canonical.svg"
MANIFEST_FILENAME = "manifest.json"
_REVISION_SHA_RE = re.compile(r"^[0-9a-f]{64}$")


class RevisionError(ValueError):
    """Raised when a revision cannot be written or resolved safely."""


def _revision_sha(track_id: str, canonical_bytes: bytes, parent_sha: str | None) -> str:
    digest = hashlib.sha256()
    digest.update(track_id.encode("utf-8"))
    digest.update(b"\x00")
    if parent_sha is not None:
        digest.update(parent_sha.encode("utf-8"))
    digest.update(b"\x00")
    digest.update(canonical_bytes)
    return digest.hexdigest()


def tip_revision_sha(tracks_dir: Path, track_id: str) -> str | None:
    """Return the head revision sha for a track, or None when no revision exists."""
    revisions = Path(tracks_dir) / track_id / "revisions"
    if not revisions.is_dir():
        return None
    shas: set[str] = set()
    parents: set[str] = set()
    for entry in revisions.iterdir():
        if not entry.is_dir() or not _REVISION_SHA_RE.match(entry.name):
            continue
        shas.add(entry.name)
        try:
            manifest = json.loads((entry / MANIFEST_FILENAME).read_text(encoding="utf-8"))
        except (OSError, ValueError):
            continue
        parent = manifest.get("parent_revision_sha256")
        if isinstance(parent, str):
            parents.add(parent)
    children = {sha for sha in shas if sha not in parents}
    if not children:
        return None
    if len(children) > 1:
        raise RevisionError(
            f"revision chain for {track_id!r} has multiple heads; "
            "refusing to guess the tip"
        )
    return children.pop()


def tip_revision_canonical_path(tracks_dir: Path, track_id: str) -> Path | None:
    """Return the canonical SVG path of a track's tip revision, or None."""
    sha = tip_revision_sha(tracks_dir, track_id)
    if sha is None:
        return None
    path = Path(tracks_dir) / track_id / "revisions" / sha / CANONICAL_FILENAME
    return path if path.is_file() else None


class TrackSession:
    def __init__(
        self,
        source: Path,
        workspace: Path,
        *,
        repo_root: Path,
        tracks_dir: Path | None = None,
        git_repo_root: Path | None = None,
        git_author=None,
    ):
        self.source = Path(source)
        self.workspace = Path(workspace)
        self.repo_root = Path(repo_root)
        self.tracks_dir = Path(tracks_dir) if tracks_dir is not None else None
        self._git_service = (
            GitScopedService(git_repo_root, author=git_author)
            if git_repo_root is not None
            else None
        )
        self.undo: list[str] = []
        self.redo: list[str] = []
        self._source_sha256 = sha256_bytes(self.source.read_bytes())
        self._load()

    def _load(self) -> None:
        self.workspace.parent.mkdir(parents=True, exist_ok=True)
        if not self.workspace.exists():
            self.workspace.write_bytes(self.source.read_bytes())
        self.current = self.workspace.read_text(encoding="utf-8")

    # -- mutation ----------------------------------------------------------

    def save(self, svg_text: str) -> dict:
        """Sanitize payload, persist canonical SVG, update session. Returns state."""
        return self._save(svg_text, source_revision=None)

    def revert(self, revision_sha: str) -> dict:
        """Restore a prior revision additively; returns state.

        Loads the old canonical content and saves it as a brand-new child
        revision. Never runs ``git reset`` or ``git checkout`` and never stages
        or commits anything outside the new revision directory.
        """
        if not _REVISION_SHA_RE.match(revision_sha):
            raise RevisionError(f"invalid revision sha {revision_sha!r}")
        if self.tracks_dir is None:
            raise RevisionError("revert requires a tracks_dir")
        content = self._read_revision_canonical(revision_sha)
        return self._save(content, source_revision=revision_sha)

    def _save(self, svg_text: str, *, source_revision: str | None) -> dict:
        canonical = sanitize_svg(svg_text.encode("utf-8"))
        persisted = canonical_xml(canonical).decode("utf-8")
        track_id = canonical.get("data-track-id", "untitled")
        changed = persisted != self.current

        if changed and self.tracks_dir is not None:
            self._commit_revision(persisted, track_id, source_revision=source_revision)
        if changed:
            self.undo.append(self.current)
            if len(self.undo) > 256:
                self.undo.pop(0)
            self.redo.clear()
            self.current = persisted
            self.workspace.write_text(persisted, encoding="utf-8")

        return self.state()

    def undo_step(self) -> dict:
        if not self.undo:
            return self.state()
        self.redo.append(self.current)
        self.current = self.undo.pop()
        self.workspace.write_text(self.current, encoding="utf-8")
        return self.state()

    def apply_region_operation(self, operation: str, region_id: str, params: dict) -> dict:
        """Apply a region operation to the current document and persist it.

        The result is sanitized, normalized and saved (creating a revision when
        content changed) so the editor can mutate large vegetation regions
        without round-tripping thousands of instances through the UI.
        """
        try:
            canonical = sanitize_svg(self.current.encode("utf-8"))
        except SVGSanitizeError as exc:
            return {"ok": False, "stage": "sanitize", "diagnostics": exc.diagnostics}
        try:
            updated = apply_region_operation(canonical, operation, region_id, params)
        except RegionOperationError as exc:
            return {"ok": False, "stage": "region", "diagnostics": [str(exc)]}
        try:
            canonical_xml(updated)
            normalize(updated)
        except (NormalizeError, SVGSanitizeError) as exc:
            diagnostics = getattr(exc, "diagnostics", None) or [str(exc)]
            return {"ok": False, "stage": "normalize", "diagnostics": diagnostics}
        result = self._save(canonical_xml(updated).decode("utf-8"), source_revision=None)
        return {"ok": True, "stage": "applied", **result}

    def redo_step(self) -> dict:
        if not self.redo:
            return self.state()
        self.undo.append(self.current)
        self.current = self.redo.pop()
        self.workspace.write_text(self.current, encoding="utf-8")
        return self.state()

    # -- immutable revisions -----------------------------------------------

    @staticmethod
    def _rmtree(path: Path) -> None:
        if not path.exists():
            return
        for child in path.iterdir():
            if child.is_dir():
                TrackSession._rmtree(child)
            else:
                child.unlink()
        path.rmdir()

    def _commit_revision(
        self,
        persisted: str,
        track_id: str,
        *,
        source_revision: str | None,
    ) -> str:
        """Write one immutable revision and its scoped Git commit."""
        if not TRACK_ID_RE.match(track_id):
            raise RevisionError(
                f"unsafe track id {track_id!r} (must match {TRACK_ID_RE.pattern})"
            )
        canonical_bytes = persisted.encode("utf-8")
        parent_sha = self._tip_revision_sha(track_id)
        revision_sha = _revision_sha(track_id, canonical_bytes, parent_sha)
        revision_dir = self.tracks_dir / track_id / "revisions" / revision_sha
        revision_dir.mkdir(parents=True, exist_ok=True)

        canonical_path = revision_dir / CANONICAL_FILENAME
        manifest_path = revision_dir / MANIFEST_FILENAME
        canonical_path.write_bytes(canonical_bytes)
        manifest_path.write_text(
            json.dumps(
                self._manifest(track_id, revision_sha, parent_sha, canonical_bytes,
                               source_revision=source_revision),
                indent=2, sort_keys=True, separators=(",", ":"),
            ),
            encoding="utf-8",
        )

        if self._git_service is not None:
            try:
                self._git_service.commit_scoped(
                    [canonical_path, manifest_path],
                    message=f"track {track_id}: revision {revision_sha}",
                )
            except Exception:
                # The commit failed after the files were written: remove the
                # orphaned revision so no uncommitted state leaks into the tree.
                self._rmtree(revision_dir)
                raise
        return revision_sha

    def _manifest(
        self,
        track_id: str,
        revision_sha: str,
        parent_sha: str | None,
        canonical_bytes: bytes,
        *,
        source_revision: str | None,
    ) -> dict:
        body = {
            "schema_version": REVISION_SCHEMA_VERSION,
            "track_id": track_id,
            "revision_sha256": revision_sha,
            "parent_revision_sha256": parent_sha,
            "source_revision_sha256": source_revision,
            "created_unix_ns": time.time_ns(),
            "artifacts": {
                "canonical": {"file": CANONICAL_FILENAME, "sha256": sha256_bytes(canonical_bytes)},
            },
        }
        body_bytes = json.dumps(body, indent=2, sort_keys=True, separators=(",", ":")).encode("utf-8")
        manifest = dict(body)
        manifest["manifest_sha256"] = sha256_bytes(body_bytes)
        return manifest

    def _read_revision_canonical(self, revision_sha: str) -> str:
        if self.tracks_dir is None:
            raise RevisionError("revisions require a tracks_dir")
        track_id = self._track_id()
        path = self.tracks_dir / track_id / "revisions" / revision_sha / CANONICAL_FILENAME
        if not path.is_file():
            raise RevisionError(f"revision {revision_sha} not found for track {track_id!r}")
        return path.read_text(encoding="utf-8")

    def _tip_revision_sha(self, track_id: str) -> str | None:
        if self.tracks_dir is None:
            return None
        return tip_revision_sha(self.tracks_dir, track_id)

    def _revision_count(self, track_id: str) -> int:
        if self.tracks_dir is None:
            return 0
        revisions = self.tracks_dir / track_id / "revisions"
        if not revisions.is_dir():
            return 0
        return sum(
            1 for entry in revisions.iterdir()
            if entry.is_dir() and _REVISION_SHA_RE.match(entry.name)
        )

    def revision_count(self, track_id: str) -> int:
        return self._revision_count(track_id)

    # -- inspection --------------------------------------------------------

    def validate(self) -> dict:
        try:
            canonical = sanitize_svg(self.current.encode("utf-8"))
        except SVGSanitizeError as exc:
            return {"ok": False, "stage": "sanitize", "diagnostics": exc.diagnostics}
        return self._report_for_canonical(canonical)

    @staticmethod
    def _report_for_canonical(canonical: ET.Element) -> dict:
        """Normalize a canonical tree and summarize collision invariants."""
        try:
            result = normalize(canonical)
        except NormalizeError as exc:
            return {"ok": False, "stage": "normalize", "diagnostics": [str(exc)]}
        checks = result["collision_validation"]
        invariants = {
            "finite_vertices": bool(checks["finite_vertices"]),
            "nondegenerate_triangles": bool(checks["nondegenerate_triangles"]),
            "blender_winding_upward": bool(checks["blender_winding_upward"]),
            "continuous_collision_grid": bool(checks["continuous_collision_grid"]),
            "safety_floor_valid": bool(checks["safety_floor_valid"]),
            "seam_continuous": float(checks["max_collision_seam_error_m"]) <= 1e-6,
        }
        return {
            "ok": all(invariants.values()),
            "stage": "validated",
            "track_id": result["track_id"],
            "centerline_length_m": result["centerline"]["length_m"],
            "banking_count": len(result["banking"]),
            "terrain_zone_count": len(result["terrain_zones"]),
            "barrier_count": len(result["barriers"]),
            "asset_count": len(result["assets"]),
            "invariants": invariants,
        }

    @staticmethod
    def import_svg(svg_text: str) -> dict:
        """Sanitize and canonicalize untrusted SVG without writing anything.

        Unsafe/unsupported input is rejected with actionable diagnostics
        (``ok=false``). Accepted input returns the canonical document plus a
        validation report; nothing is persisted until the editor saves.
        """
        try:
            canonical = sanitize_svg(svg_text.encode("utf-8"))
        except SVGSanitizeError as exc:
            return {"ok": False, "stage": "sanitize", "diagnostics": exc.diagnostics}
        canonical_text = canonical_xml(canonical).decode("utf-8")
        validation = TrackSession._report_for_canonical(canonical)
        return {
            "ok": True,
            "stage": "imported",
            "canonical": canonical_text,
            "validation": validation,
        }

    def state(self) -> dict:
        track_id = self._track_id()
        return {
            "track_id": track_id,
            "source_sha256": self._source_sha256,
            "undo_depth": len(self.undo),
            "redo_depth": len(self.redo),
            "current": self.current,
            "current_revision_sha256": self._tip_revision_sha(track_id),
            "revision_count": self._revision_count(track_id),
        }

    @property
    def undo_depth(self) -> int:
        return len(self.undo)

    @property
    def redo_depth(self) -> int:
        return len(self.redo)

    def _track_id(self) -> str:
        try:
            return sanitize_svg(self.current.encode("utf-8")).get("data-track-id", "untitled")
        except SVGSanitizeError:
            return "invalid"
