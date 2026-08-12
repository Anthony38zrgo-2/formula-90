"""Scoped Git operations for the Track Authoring System.

Stages and commits only the files of a single track revision, so unrelated
dirty worktree changes are never staged, altered, or committed. This service
never runs reset or checkout; restoring an older revision is always additive
because it writes a brand-new revision directory before committing.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import subprocess


class GitServiceError(RuntimeError):
    """Raised when a scoped Git operation cannot be completed safely."""


@dataclass(frozen=True)
class GitAuthor:
    name: str
    email: str


DEFAULT_AUTHOR = GitAuthor("f90-track-authoring", "f90-track-authoring@local")


class GitScopedService:
    def __init__(self, repo_root: Path, *, author: GitAuthor | None = None):
        self.repo_root = Path(repo_root)
        self.author = author or DEFAULT_AUTHOR

    def _run(self, *args: str) -> subprocess.CompletedProcess[str]:
        command = ["git", "-C", str(self.repo_root), *args]
        result = subprocess.run(command, capture_output=True, text=True)
        if result.returncode != 0:
            raise GitServiceError(
                f"git command failed: {' '.join(command)}\n{result.stderr.strip()}"
            )
        return result

    def is_repository(self) -> bool:
        result = subprocess.run(
            ["git", "-C", str(self.repo_root), "rev-parse", "--is-inside-work-tree"],
            capture_output=True,
            text=True,
        )
        return result.returncode == 0

    def commit_scoped(self, paths: list[Path], message: str) -> str:
        """Stage and commit exactly ``paths``; returns the new HEAD sha.

        Only the given files are added and committed. Everything else in the
        worktree, including unrelated dirty files and unrelated staged changes,
        is left untouched.
        """
        if not self.is_repository():
            raise GitServiceError(f"not a git work tree: {self.repo_root}")
        relative = []
        for path in paths:
            path = Path(path)
            try:
                relative.append(str(path.relative_to(self.repo_root)).replace("\\", "/"))
            except ValueError as exc:
                raise GitServiceError(f"path outside repository root: {path}") from exc
        self._run("add", "--", *relative)
        self._run(
            "-c", f"user.name={self.author.name}",
            "-c", f"user.email={self.author.email}",
            "-c", "commit.gpgsign=false",
            "commit", "-m", message, "--", *relative,
        )
        return self.head_sha()

    def head_sha(self) -> str:
        return self._run("rev-parse", "HEAD").stdout.strip()
