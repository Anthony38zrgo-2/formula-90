"""Fail-closed output path policy for preview and promotion commands.

This module validates destinations only; it never creates, moves, or deletes
files. Callers must validate immediately before performing a write.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from enum import Enum
from pathlib import Path
from typing import Sequence


class OutputPolicyError(ValueError):
    """Raised when an output destination violates an architecture boundary."""


class OutputMode(str, Enum):
    """Explicit write modes supported by the architecture."""

    PREVIEW = "preview"
    PROMOTE = "promote"


@dataclass(frozen=True)
class OutputDecision:
    """Resolved destination and the allowed root that contains it."""

    mode: OutputMode
    path: Path
    allowed_root: Path


def _resolved_path(repo_root: Path, value: str | Path) -> Path:
    candidate = Path(value)
    if not candidate.is_absolute():
        candidate = repo_root / candidate
    return candidate.resolve(strict=False)


def _is_strict_descendant(candidate: Path, root: Path) -> bool:
    return candidate != root and root in candidate.parents


def validate_output_path(
    repo_root: str | Path,
    output_path: str | Path,
    mode: str | OutputMode,
) -> OutputDecision:
    """Validate an output path against Formula90s architecture boundaries.

    Preview writes are restricted to descendants of ``scratch/``. Promotion
    writes are restricted to descendants of ``game/assets/`` or
    ``game/sounds/``. The roots themselves are rejected to prevent broad
    replace/delete operations.
    """

    root = Path(repo_root).resolve(strict=True)
    if not root.is_dir():
        raise OutputPolicyError(f"Repository root is not a directory: {root}")

    try:
        output_mode = mode if isinstance(mode, OutputMode) else OutputMode(mode)
    except ValueError as error:
        expected = ", ".join(item.value for item in OutputMode)
        raise OutputPolicyError(f"Unknown output mode {mode!r}; expected: {expected}") from error

    destination = _resolved_path(root, output_path)
    if output_mode is OutputMode.PREVIEW:
        allowed_roots = (root / "scratch",)
    else:
        allowed_roots = (root / "game" / "assets", root / "game" / "sounds")

    resolved_roots = tuple(path.resolve(strict=False) for path in allowed_roots)
    for allowed_root in resolved_roots:
        if _is_strict_descendant(destination, allowed_root):
            return OutputDecision(output_mode, destination, allowed_root)

    allowed = ", ".join(str(path) for path in resolved_roots)
    raise OutputPolicyError(
        f"{output_mode.value} output must be a child of: {allowed}; got: {destination}"
    )


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Validate a Formula90s preview or promotion destination without writing it."
    )
    parser.add_argument("--repo", type=Path, required=True, help="Formula90s repository root")
    parser.add_argument("--mode", choices=[item.value for item in OutputMode], required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    try:
        decision = validate_output_path(args.repo, args.output, args.mode)
    except (OSError, OutputPolicyError) as error:
        parser.exit(2, f"output-policy: {error}\n")
    print(decision.path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
