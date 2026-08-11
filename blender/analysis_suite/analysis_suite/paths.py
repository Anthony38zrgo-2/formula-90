"""Repository-root discovery and repository-relative path handling.

Configs, reports and docs use repository-relative paths. They resolve against
the repository root discovered by walking upward from the executing script,
never against the current working directory.
"""
import os
from pathlib import Path

_ROOT_CACHE = {}


def find_repository_root(start=None):
    """Walk upward from ``start`` looking for a ``.git`` marker.

    ``.git`` may be a directory (regular clone) or a file (worktree pointer),
    so existence is the only check. Results are cached per start path.
    """
    if start is None:
        start = Path(__file__).resolve().parent
    key = str(Path(start).resolve())
    if key in _ROOT_CACHE:
        return _ROOT_CACHE[key]
    candidate = Path(start).resolve()
    if not candidate.is_dir():
        candidate = candidate.parent
    while True:
        if (candidate / '.git').exists():
            _ROOT_CACHE[key] = candidate
            return candidate
        parent = candidate.parent
        if parent == candidate:
            raise RuntimeError('repository root not found (no .git marker)')
        candidate = parent


def repo_relative(root, path):
    """Return a POSIX-style repository-relative path, or None if outside."""
    root = os.path.normpath(os.path.abspath(str(root)))
    path = os.path.normpath(os.path.abspath(str(path)))
    try:
        rel = os.path.relpath(path, root)
    except ValueError:  # different drive on Windows
        return None
    if rel == '..' or rel.startswith('..' + os.sep):
        return None
    return rel.replace(os.sep, '/')


def resolve_repo_path(root, rel):
    """Resolve a repository-relative path and forbid escaping the root."""
    root = os.path.normpath(os.path.abspath(str(root)))
    joined = os.path.normpath(os.path.join(root, rel))
    if not (joined == root or joined.startswith(root + os.sep)):
        raise ValueError('path escapes the repository root: %s' % rel)
    return Path(joined)


def resolve_input(root, value, extension=None, label='source'):
    """Resolve a config input: repository-relative or absolute under root.

    ``extension`` (e.g. '.glb') enforces a supported input type; anything else
    is rejected as an unsupported extension (exit code 2 class).
    """
    from .contracts import ConfigError, require_str

    value = require_str(value, label)
    if os.path.isabs(value):
        path = Path(os.path.normpath(value))
    else:
        try:
            path = resolve_repo_path(root, value)
        except ValueError as exc:
            raise ConfigError(str(exc)) from exc
    if extension is not None:
        if path.suffix.lower() != extension.lower():
            raise ConfigError(
                '%s must reference a .%s file, got: %s'
                % (label, extension.lstrip('.'), value)
            )
    if not path.exists():
        raise ConfigError('%s does not exist: %s' % (label, value))
    if not path.is_file():
        raise ConfigError('%s is not a file: %s' % (label, value))
    return path
