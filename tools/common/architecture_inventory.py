"""Deterministic static inventory of Formula90s architecture boundary debt.

The scanner is intentionally heuristic and non-blocking. It reports candidates
for migration planning; it does not claim that every finding is a defect.
"""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable, Sequence


SCHEMA_VERSION = 1
DEFAULT_SCAN_ROOTS = ("blender", "tools", "game", "scripts", "tests", "native")
TEXT_EXTENSIONS = {
    ".cfg", ".gd", ".gdextension", ".json", ".ps1", ".py", ".rs",
    ".toml", ".ts", ".tscn", ".tsx", ".yaml", ".yml",
}
EXCLUDED_PARTS = {
    ".codex-native", ".codex-target", ".codex_tmp", ".git", ".godot",
    ".pytest_cache", ".ruff_cache", ".tmp", ".tools", ".venv",
    "__pycache__", "build", "coverage", "dist", "node_modules", "reports",
    "scratch", "target", "third_party", "tmp",
}
EXCLUDED_FILE_NAMES = {
    "architecture_inventory.py",
    "output_policy.py",
    "test_architecture_inventory.py",
    "test_output_policy.py",
}
EXCLUDED_PREFIXES = (
    "blender/generated/",
    "game/resources/environment/generated/",
)
MAX_TEXT_BYTES = 2 * 1024 * 1024

PATH_PATTERNS = (
    re.compile(r"game[\\/](?:assets|sounds|resources)(?:[\\/]|\b)", re.IGNORECASE),
    re.compile(r"assets-lowpoly-python(?:[\\/]|\b)", re.IGNORECASE),
    re.compile(r"blender[\\/]track_pipeline(?:[\\/]|\b)", re.IGNORECASE),
)
SOURCE_PATH_PATTERN = re.compile(
    r"(?:assets-lowpoly-python|blender[\\/](?:assets|track_pipeline)|source-assets)(?:[\\/]|\b)",
    re.IGNORECASE,
)
RUNTIME_PATH_PATTERN = re.compile(
    r"game[\\/](?:assets|sounds|resources)(?:[\\/]|\b)", re.IGNORECASE
)
PARENT_ARITHMETIC_PATTERN = re.compile(r"\.parents\s*\[\s*\d+\s*\]")
OUTPUT_DEFAULT_PATTERN = re.compile(
    r"(?:DEFAULT_OUTPUT|runtime_dir|output_dir|destination|--output)", re.IGNORECASE
)
WRITE_OPERATION_PATTERN = re.compile(
    r"(?:\.write_(?:text|bytes)\s*\(|"
    r"\bopen\s*\([^\n]*(?:['\"](?:w|a|x)[bt+]*['\"]|mode\s*=)|"
    r"\b(?:shutil\.)?(?:copy|copy2|copyfile|copytree|move)\s*\(|"
    r"\bos\.(?:replace|rename)\s*\(|"
    r"\b(?:Set-Content|Copy-Item|Move-Item|New-Item)\b)",
    re.IGNORECASE,
)


@dataclass(frozen=True)
class Finding:
    kind: str
    path: str
    line: int
    evidence: str
    related_line: int | None = None


def _short_evidence(line: str) -> str:
    return " ".join(line.strip().split())[:240]


def _is_scannable(repo_root: Path, path: Path) -> bool:
    relative = path.relative_to(repo_root).as_posix()
    return (
        path.is_file()
        and path.suffix.lower() in TEXT_EXTENSIONS
        and path.name not in EXCLUDED_FILE_NAMES
        and not relative.startswith(EXCLUDED_PREFIXES)
        and not any(part in EXCLUDED_PARTS for part in path.parts)
        and path.stat().st_size <= MAX_TEXT_BYTES
    )


def _iter_files(repo_root: Path, scan_roots: Iterable[str]) -> Iterable[Path]:
    files: list[Path] = []
    for relative_root in scan_roots:
        candidate = repo_root / relative_root
        if candidate.is_file() and _is_scannable(repo_root, candidate):
            files.append(candidate)
        elif candidate.is_dir():
            files.extend(
                path for path in candidate.rglob("*") if _is_scannable(repo_root, path)
            )
    return sorted(set(files), key=lambda path: path.relative_to(repo_root).as_posix())


def _scan_file(repo_root: Path, path: Path) -> list[Finding]:
    relative = path.relative_to(repo_root).as_posix()
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    findings: list[Finding] = []
    runtime_references: list[tuple[int, str]] = []
    write_operations: list[tuple[int, str]] = []

    for line_number, line in enumerate(lines, start=1):
        if any(pattern.search(line) for pattern in PATH_PATTERNS):
            findings.append(Finding("cross_zone_literal", relative, line_number, _short_evidence(line)))
        if PARENT_ARITHMETIC_PATTERN.search(line):
            findings.append(Finding("parent_arithmetic", relative, line_number, _short_evidence(line)))
        if RUNTIME_PATH_PATTERN.search(line):
            runtime_references.append((line_number, line))
            if OUTPUT_DEFAULT_PATTERN.search(line):
                findings.append(
                    Finding("runtime_output_default", relative, line_number, _short_evidence(line))
                )
        if WRITE_OPERATION_PATTERN.search(line):
            write_operations.append((line_number, line))
        if relative.startswith("game/") and SOURCE_PATH_PATTERN.search(line):
            findings.append(
                Finding("game_source_dependency", relative, line_number, _short_evidence(line))
            )

    if runtime_references and write_operations:
        runtime_line, _runtime_evidence = runtime_references[0]
        write_line, write_evidence = write_operations[0]
        findings.append(
            Finding(
                "runtime_write_candidate",
                relative,
                write_line,
                _short_evidence(write_evidence),
                related_line=runtime_line,
            )
        )
    return findings


def build_inventory(repo_root: str | Path, scan_roots: Sequence[str] = DEFAULT_SCAN_ROOTS) -> dict:
    root = Path(repo_root).resolve(strict=True)
    if not root.is_dir():
        raise ValueError(f"Repository root is not a directory: {root}")
    files = list(_iter_files(root, scan_roots))
    findings = [finding for path in files for finding in _scan_file(root, path)]
    findings.sort(key=lambda item: (item.kind, item.path, item.line, item.related_line or 0))
    counts = Counter(finding.kind for finding in findings)
    return {
        "schema_version": SCHEMA_VERSION,
        "scan_roots": list(scan_roots),
        "scanned_file_count": len(files),
        "finding_count": len(findings),
        "counts_by_kind": dict(sorted(counts.items())),
        "findings": [asdict(finding) for finding in findings],
    }


def serialize_inventory(inventory: dict) -> str:
    return json.dumps(inventory, indent=2, sort_keys=True, ensure_ascii=False) + "\n"


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Generate a deterministic, non-blocking architecture boundary inventory."
    )
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--scan-root", action="append", dest="scan_roots")
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _build_parser().parse_args(argv)
    scan_roots = tuple(args.scan_roots) if args.scan_roots else DEFAULT_SCAN_ROOTS
    serialized = serialize_inventory(build_inventory(args.repo, scan_roots))
    if args.output:
        output = args.output if args.output.is_absolute() else args.repo / args.output
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(serialized, encoding="utf-8", newline="\n")
        print(output.resolve())
    else:
        print(serialized, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
