"""Explicit, recoverable preview and content promotion operations."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import uuid
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Sequence

try:
    from .output_policy import OutputMode, OutputPolicyError, validate_output_path
except ImportError:  # Direct CLI execution.
    from output_policy import OutputMode, OutputPolicyError, validate_output_path


class ContentFlowError(ValueError):
    """Raised when a preview or promotion cannot proceed safely."""


@dataclass(frozen=True)
class TransferResult:
    mode: str
    source: str
    output: str
    replaced: bool
    backup: str | None
    planned: bool


def _relative(repo_root: Path, path: Path) -> str:
    return path.relative_to(repo_root).as_posix()


def _strict_descendant(path: Path, root: Path) -> bool:
    return path != root and root in path.parents


def _resolve_source(repo_root: Path, source: str | Path, *, require_scratch: bool) -> Path:
    raw = Path(source)
    if not raw.is_absolute():
        raw = repo_root / raw
    if raw.is_symlink():
        raise ContentFlowError(f"Source symlinks are not allowed: {raw}")
    try:
        resolved = raw.resolve(strict=True)
    except FileNotFoundError as error:
        raise ContentFlowError(f"Source does not exist: {raw}") from error
    if not _strict_descendant(resolved, repo_root):
        raise ContentFlowError(f"Source must be inside the repository: {resolved}")
    if require_scratch:
        scratch = (repo_root / "scratch").resolve(strict=True)
        if not _strict_descendant(resolved, scratch):
            raise ContentFlowError(f"Promotion source must be under scratch: {resolved}")
    _reject_symlinks(resolved)
    return resolved


def _reject_symlinks(source: Path) -> None:
    if source.is_symlink():
        raise ContentFlowError(f"Source symlinks are not allowed: {source}")
    if source.is_dir():
        for child in source.rglob("*"):
            if child.is_symlink():
                raise ContentFlowError(f"Source tree contains a symlink: {child}")


def _reject_overlap(source: Path, output: Path) -> None:
    if source == output or source in output.parents or output in source.parents:
        raise ContentFlowError(f"Source and output must not overlap: {source} -> {output}")


def _remove_path(path: Path) -> None:
    if path.is_dir() and not path.is_symlink():
        shutil.rmtree(path)
    elif path.exists() or path.is_symlink():
        path.unlink()


def _copy_path(source: Path, output: Path) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    if source.is_dir():
        shutil.copytree(source, output)
    else:
        shutil.copy2(source, output)


def _transaction_paths(repo_root: Path, transaction_id: str) -> tuple[Path, Path]:
    transaction_root = repo_root / "scratch" / ".content-flow-staging" / transaction_id
    payload = transaction_root / "payload"
    validate_output_path(repo_root, payload, OutputMode.PREVIEW)
    return transaction_root, payload


def _result(
    repo_root: Path,
    mode: OutputMode,
    source: Path,
    output: Path,
    replaced: bool,
    backup: Path | None,
    planned: bool,
) -> TransferResult:
    return TransferResult(
        mode=mode.value,
        source=_relative(repo_root, source),
        output=_relative(repo_root, output),
        replaced=replaced,
        backup=_relative(repo_root, backup) if backup else None,
        planned=planned,
    )


def preview(
    repo_root: str | Path,
    source: str | Path,
    output: str | Path,
    *,
    replace: bool = False,
    plan: bool = False,
    transaction_id: str | None = None,
) -> TransferResult:
    root = Path(repo_root).resolve(strict=True)
    resolved_source = _resolve_source(root, source, require_scratch=False)
    decision = validate_output_path(root, output, OutputMode.PREVIEW)
    destination = decision.path
    _reject_overlap(resolved_source, destination)
    exists = destination.exists() or destination.is_symlink()
    if exists and not replace:
        raise ContentFlowError(f"Preview output exists; pass --replace: {destination}")
    if plan:
        return _result(root, OutputMode.PREVIEW, resolved_source, destination, exists, None, True)

    tx = transaction_id or uuid.uuid4().hex
    transaction_root, staged = _transaction_paths(root, tx)
    try:
        _copy_path(resolved_source, staged)
        destination.parent.mkdir(parents=True, exist_ok=True)
        if exists:
            _remove_path(destination)
        os.replace(staged, destination)
    finally:
        if transaction_root.exists():
            shutil.rmtree(transaction_root)
    return _result(root, OutputMode.PREVIEW, resolved_source, destination, exists, None, False)


def promote(
    repo_root: str | Path,
    source: str | Path,
    output: str | Path,
    *,
    replace: bool = False,
    plan: bool = False,
    transaction_id: str | None = None,
) -> TransferResult:
    root = Path(repo_root).resolve(strict=True)
    resolved_source = _resolve_source(root, source, require_scratch=True)
    decision = validate_output_path(root, output, OutputMode.PROMOTE)
    destination = decision.path
    _reject_overlap(resolved_source, destination)
    exists = destination.exists() or destination.is_symlink()
    if exists and not replace:
        raise ContentFlowError(f"Runtime output exists; pass --replace: {destination}")
    if plan:
        return _result(root, OutputMode.PROMOTE, resolved_source, destination, exists, None, True)

    tx = transaction_id or uuid.uuid4().hex
    transaction_root, staged = _transaction_paths(root, tx)
    backup = root / "scratch" / "promote-backups" / tx / destination.name if exists else None
    if backup:
        validate_output_path(root, backup, OutputMode.PREVIEW)

    installed = False
    try:
        _copy_path(resolved_source, staged)
        destination.parent.mkdir(parents=True, exist_ok=True)
        if backup:
            backup.parent.mkdir(parents=True, exist_ok=False)
            os.replace(destination, backup)
        os.replace(staged, destination)
        installed = True
    except Exception:
        if installed and destination.exists():
            _remove_path(destination)
        if backup and backup.exists() and not destination.exists():
            destination.parent.mkdir(parents=True, exist_ok=True)
            os.replace(backup, destination)
        raise
    finally:
        if transaction_root.exists():
            shutil.rmtree(transaction_root)
    return _result(root, OutputMode.PROMOTE, resolved_source, destination, exists, backup, False)


def _add_transfer_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--replace", action="store_true")
    parser.add_argument("--plan", action="store_true")


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Formula90s preview/promote content flow")
    commands = parser.add_subparsers(dest="command", required=True)
    _add_transfer_arguments(commands.add_parser("preview"))
    _add_transfer_arguments(commands.add_parser("promote"))
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _build_parser().parse_args(argv)
    operation = preview if args.command == "preview" else promote
    try:
        result = operation(
            args.repo,
            args.source,
            args.output,
            replace=args.replace,
            plan=args.plan,
        )
    except (ContentFlowError, OSError, OutputPolicyError) as error:
        raise SystemExit(f"content-flow: {error}") from error
    print(json.dumps(asdict(result), indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
