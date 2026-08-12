"""General Asset Registry for the F90 Track Authoring System.

Tracks reference assets by semantic ID, never by filesystem path. The registry
owns: semantic asset IDs, allowed kinds, source paths, dimensions, previews,
collision class, placement budget and provenance (source hashes).

It validates duplicate IDs, nonexistent sources/previews, invalid kinds,
collision classes, degenerate dimensions and inconsistent budgets. It never
modifies asset geometry.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
import hashlib
import json
import re

ASSET_ID_RE = re.compile(r"^[a-z0-9][a-z0-9_.-]{0,63}$")

ALLOWED_KINDS = {"card", "flag", "vegetation", "building", "barrier_visual", "prop"}
ALLOWED_COLLISION_CLASSES = {"none", "collision"}

SCHEMA_VERSION = 1
PROFILE_VERSION = "f90-track-1"

# A source is required for every kind except procedural generators.
PROCEDURAL_KINDS = {"flag"}


@dataclass(frozen=True)
class AssetSpec:
    id: str
    kind: str
    category: str
    source: str | None
    dimensions_m: dict[str, float] = field(default_factory=dict)
    preview: str | None = None
    source_sha256: str | None = None
    collision_class: str = "none"
    budget: dict[str, int] = field(default_factory=lambda: {"min_instances": 0, "max_instances": 0})
    metadata: dict = field(default_factory=dict)


class AssetRegistry:
    def __init__(self, specs: list[AssetSpec], *, repo_root: Path):
        self.specs = list(specs)
        self._by_id: dict[str, AssetSpec] = {spec.id: spec for spec in self.specs}
        self.repo_root = Path(repo_root)

    def lookup(self, asset_id: str) -> AssetSpec | None:
        return self._by_id.get(asset_id)

    def ids(self) -> list[str]:
        return [spec.id for spec in self.specs]

    def by_kind(self, kind: str) -> list[AssetSpec]:
        return [spec for spec in self.specs if spec.kind == kind]

    def by_category(self, category: str) -> list[AssetSpec]:
        return [spec for spec in self.specs if spec.category == category]

    def __contains__(self, asset_id: str) -> bool:
        return asset_id in self._by_id

    def __len__(self) -> int:
        return len(self.specs)


def parse_registry(data: dict, *, repo_root: Path) -> AssetRegistry:
    specs = []
    for raw in data.get("assets", []):
        specs.append(
            AssetSpec(
                id=raw["id"],
                kind=raw.get("kind", ""),
                category=raw.get("category", ""),
                source=raw.get("source"),
                dimensions_m=dict(raw.get("dimensions_m", {})),
                preview=raw.get("preview"),
                source_sha256=raw.get("source_sha256"),
                collision_class=raw.get("collision_class", "none"),
                budget=dict(raw.get("budget", {})),
                metadata=dict(raw.get("metadata", {})),
            )
        )
    return AssetRegistry(specs, repo_root=repo_root)


def load_registry(path: str | Path, *, repo_root: str | Path) -> AssetRegistry:
    return parse_registry(json.loads(Path(path).read_text(encoding="utf-8")), repo_root=Path(repo_root))


def _sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_registry(registry: AssetRegistry) -> list[str]:
    """Return a list of diagnostics; an empty list means the registry is valid."""
    errors: list[str] = []
    seen: set[str] = set()

    for spec in registry.specs:
        ident = spec.id
        if not ident:
            errors.append("asset: empty id")
        elif not ASSET_ID_RE.match(ident):
            errors.append(f"asset {ident!r}: invalid id (use lowercase [a-z0-9_.-])")
        if ident in seen:
            errors.append(f"asset {ident!r}: duplicate id")
        seen.add(ident)

        if spec.kind not in ALLOWED_KINDS:
            errors.append(f"asset {ident!r}: unknown kind {spec.kind!r} (allowed: {sorted(ALLOWED_KINDS)})")
        if spec.collision_class not in ALLOWED_COLLISION_CLASSES:
            errors.append(f"asset {ident!r}: unknown collision_class {spec.collision_class!r}")

        if spec.source:
            source = registry.repo_root / spec.source
            if not source.exists():
                errors.append(f"asset {ident!r}: missing source {spec.source}")
            elif spec.source_sha256:
                if _sha256_file(source) != spec.source_sha256:
                    errors.append(f"asset {ident!r}: source sha256 mismatch for {spec.source}")
        else:
            if spec.kind not in PROCEDURAL_KINDS:
                errors.append(f"asset {ident!r}: missing source (kind {spec.kind!r} requires one)")

        if spec.preview:
            if not (registry.repo_root / spec.preview).exists():
                errors.append(f"asset {ident!r}: missing preview {spec.preview}")

        for axis, value in spec.dimensions_m.items():
            if axis not in ("width", "height", "depth"):
                errors.append(f"asset {ident!r}: unknown dimension axis {axis!r}")
            elif not (value > 0.0):
                errors.append(f"asset {ident!r}: dimension {axis} must be positive, got {value}")

        minimum = int(spec.budget.get("min_instances", 0))
        maximum = int(spec.budget.get("max_instances", 0))
        if minimum < 0 or maximum < 0:
            errors.append(f"asset {ident!r}: budget counts cannot be negative")
        if maximum > 0 and minimum > maximum:
            errors.append(f"asset {ident!r}: budget min_instances {minimum} exceeds max_instances {maximum}")

    return errors


def validate_registry_file(path: str | Path, *, repo_root: str | Path) -> list[str]:
    registry = load_registry(path, repo_root=repo_root)
    return validate_registry(registry)
