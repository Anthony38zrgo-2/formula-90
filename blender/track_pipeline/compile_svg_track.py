"""Deterministic F90 Track SVG compiler (source contract, no Blender).

Compiles a source SVG through sanitize -> canonicalize -> normalize -> asset
registry resolution/budget validation and writes only generated artifacts
beneath the requested output directory:

* ``track.canonical.svg``
* ``track.normalized.json``
* ``manifest.json`` (source/canonical/registry/compiler SHA-256 provenance)

Identical input must produce byte-identical artifacts across runs. Invalid,
unsafe, unresolved or out-of-budget input is rejected with actionable
diagnostics and nothing is written.
"""

from __future__ import annotations

import argparse
from pathlib import Path
import hashlib
import json
import sys

from asset_registry import AssetRegistry, load_registry, validate_registry
from svg_normalizer import NormalizeError, normalize
from svg_profile import PROFILE_VERSION, SCHEMA_VERSION, sha256_bytes
from svg_sanitizer import SVGSanitizeError, canonical_xml, sanitize_svg

COMPILER_NAME = "compile_svg_track"
COMPILER_VERSION = "1.0.0"

CANONICAL_ARTIFACT = "track.canonical.svg"
NORMALIZED_ARTIFACT = "track.normalized.json"
MANIFEST_ARTIFACT = "manifest.json"

_REPO_ROOT = Path(__file__).resolve().parents[2]

# Modules that define compilation behavior; the manifest pins their collective
# SHA-256 so a compiled artifact can always be traced to its compiler version.
_COMPILER_MODULES = (
    "compile_svg_track.py",
    "svg_profile.py",
    "svg_sanitizer.py",
    "svg_normalizer.py",
    "asset_registry.py",
)


class CompileError(ValueError):
    """Aggregated, actionable rejection with zero output written."""

    def __init__(self, diagnostics: list[str]):
        self.diagnostics = list(diagnostics)
        super().__init__("; ".join(self.diagnostics))


def _compiler_sha256() -> str:
    digest = hashlib.sha256()
    for name in _COMPILER_MODULES:
        digest.update(name.encode("utf-8"))
        digest.update((Path(__file__).with_name(name)).read_bytes())
    return digest.hexdigest()


def compile_track(
    source_bytes: bytes,
    *,
    source_name: str,
    registry: AssetRegistry,
    registry_path: Path,
    output_dir: Path,
) -> dict[str, object]:
    """Compile once and write the artifacts; returns the manifest dict.

    Raises :class:`CompileError` on any rejection. On success the output
    directory contains exactly the three generated artifacts.
    """
    output_dir = Path(output_dir)
    registry_path = Path(registry_path)

    registry_errors = validate_registry(registry)
    if registry_errors:
        raise CompileError(registry_errors)

    try:
        canonical = sanitize_svg(source_bytes)
    except SVGSanitizeError as exc:
        raise CompileError(exc.diagnostics) from exc

    try:
        normalized = normalize(canonical)
    except NormalizeError as exc:
        raise CompileError([str(exc)]) from exc

    _resolve_assets(normalized, registry)

    canonical_bytes = canonical_xml(canonical)
    normalized_bytes = json.dumps(
        normalized, indent=2, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")

    manifest: dict[str, object] = {
        "schema_version": SCHEMA_VERSION,
        "profile_version": PROFILE_VERSION,
        "track_id": normalized["track_id"],
        "artifacts": {
            "source": {"file": source_name, "sha256": sha256_bytes(source_bytes)},
            "canonical": {"file": CANONICAL_ARTIFACT, "sha256": sha256_bytes(canonical_bytes)},
            "normalized": {"file": NORMALIZED_ARTIFACT, "sha256": sha256_bytes(normalized_bytes)},
            "registry": {
                "file": registry_path.name,
                "sha256": sha256_bytes(registry_path.read_bytes()),
            },
        },
        "compiler": {
            "name": COMPILER_NAME,
            "version": COMPILER_VERSION,
            "sha256": _compiler_sha256(),
        },
    }

    output_dir.mkdir(parents=True, exist_ok=True)
    (output_dir / CANONICAL_ARTIFACT).write_bytes(canonical_bytes)
    (output_dir / NORMALIZED_ARTIFACT).write_bytes(normalized_bytes)
    (output_dir / MANIFEST_ARTIFACT).write_bytes(
        json.dumps(manifest, indent=2, sort_keys=True, separators=(",", ":")).encode("utf-8")
    )
    return manifest


def _resolve_assets(normalized: dict, registry: AssetRegistry) -> None:
    """Resolve every ``asset_id`` against the registry and enforce budgets."""
    diagnostics: list[str] = []
    counts: dict[str, int] = {}
    for asset in normalized["assets"]:
        spec = registry.lookup(asset["asset_id"])
        if spec is None:
            diagnostics.append(
                f"asset {asset['instance_id']!r}: unknown asset id {asset['asset_id']!r} "
                f"(registry contains {len(registry)} asset(s))"
            )
            continue
        asset["kind"] = spec.kind
        counts[asset["asset_id"]] = counts.get(asset["asset_id"], 0) + 1
    if diagnostics:
        raise CompileError(diagnostics)

    for asset_id, count in sorted(counts.items()):
        spec = registry.lookup(asset_id)
        minimum = int(spec.budget.get("min_instances", 0))
        maximum = int(spec.budget.get("max_instances", 0))
        if count < minimum:
            diagnostics.append(
                f"asset {asset_id!r}: {count} instance(s) below budget minimum {minimum}"
            )
        if maximum > 0 and count > maximum:
            diagnostics.append(
                f"asset {asset_id!r}: {count} instance(s) above budget maximum {maximum}"
            )
    if diagnostics:
        raise CompileError(diagnostics)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Compile an F90 Track SVG into canonical SVG, normalized JSON "
                    "and a provenance manifest."
    )
    parser.add_argument("--source", required=True, help="Path to the SVG track source.")
    parser.add_argument("--registry", required=True, help="Path to the Asset Registry JSON.")
    parser.add_argument("--output-dir", required=True, help="Directory for generated artifacts.")
    args = parser.parse_args(argv)

    source = Path(args.source)
    registry_path = Path(args.registry)
    if not source.is_file():
        print(f"FAIL source not found: {source}", file=sys.stderr)
        return 2
    if not registry_path.is_file():
        print(f"FAIL registry not found: {registry_path}", file=sys.stderr)
        return 2

    try:
        registry = load_registry(registry_path, repo_root=_REPO_ROOT)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"FAIL registry load: {exc}", file=sys.stderr)
        return 2

    try:
        manifest = compile_track(
            source.read_bytes(),
            source_name=source.name,
            registry=registry,
            registry_path=registry_path,
            output_dir=args.output_dir,
        )
    except CompileError as exc:
        print("FAIL compile:", file=sys.stderr)
        for diagnostic in exc.diagnostics:
            print(f"- {diagnostic}", file=sys.stderr)
        return 2

    output = Path(args.output_dir)
    print(f"track: {manifest['track_id']}")
    print(f"canonical:  {(output / CANONICAL_ARTIFACT).resolve()}  {manifest['artifacts']['canonical']['sha256']}")
    print(f"normalized: {(output / NORMALIZED_ARTIFACT).resolve()}  {manifest['artifacts']['normalized']['sha256']}")
    manifest_bytes = (output / MANIFEST_ARTIFACT).read_bytes()
    print(f"manifest:   {(output / MANIFEST_ARTIFACT).resolve()}  {sha256_bytes(manifest_bytes)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
