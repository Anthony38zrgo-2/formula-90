from __future__ import annotations

from dataclasses import dataclass
from typing import Iterable


@dataclass(frozen=True)
class ProceduralAssetSpec:
    id: str
    category: str
    weight: float
    radius_m: float
    scale_min: float
    scale_max: float


REGION_PROFILES: dict[str, dict[str, tuple[ProceduralAssetSpec, ...]]] = {
    "south_america": {
        "trees": (
            ProceduralAssetSpec("sa_broadleaf", "trees", 0.50, 2.35, 0.82, 1.22),
            ProceduralAssetSpec("sa_dry_canopy", "trees", 0.32, 2.10, 0.80, 1.18),
            ProceduralAssetSpec("sa_palm", "trees", 0.18, 1.55, 0.88, 1.16),
        ),
        "bushes": (
            ProceduralAssetSpec("sa_bush_round", "bushes", 0.60, 1.05, 0.80, 1.22),
            ProceduralAssetSpec("sa_bush_dry", "bushes", 0.40, 0.85, 0.78, 1.18),
        ),
        "grass": (
            ProceduralAssetSpec("sa_grass_clump", "grass", 0.70, 0.34, 0.75, 1.20),
            ProceduralAssetSpec("sa_grass_dry", "grass", 0.30, 0.28, 0.72, 1.15),
        ),
        "fake_buildings": (
            ProceduralAssetSpec("sa_building_low", "fake_buildings", 0.58, 6.5, 0.90, 1.10),
            ProceduralAssetSpec("sa_building_warehouse", "fake_buildings", 0.42, 9.0, 0.90, 1.08),
        ),
    }
}


def supported_regions() -> tuple[str, ...]:
    return tuple(sorted(REGION_PROFILES))


def get_region_profile(region: str) -> dict[str, tuple[ProceduralAssetSpec, ...]]:
    key = region.strip().lower()
    if key not in REGION_PROFILES:
        raise KeyError(f"Unsupported procedural region {region!r}; supported: {', '.join(supported_regions())}")
    return REGION_PROFILES[key]


def specs_for(region: str, category: str) -> tuple[ProceduralAssetSpec, ...]:
    profile = get_region_profile(region)
    if category not in profile:
        raise KeyError(f"Region {region!r} has no category {category!r}")
    return profile[category]


def spec_map(region: str) -> dict[str, ProceduralAssetSpec]:
    return {
        spec.id: spec
        for specs in get_region_profile(region).values()
        for spec in specs
    }


def weighted_choice(rng, specs: Iterable[ProceduralAssetSpec]) -> ProceduralAssetSpec:
    values = tuple(specs)
    if not values:
        raise ValueError("weighted_choice requires at least one asset spec")
    total = sum(max(0.0, spec.weight) for spec in values)
    if total <= 0.0:
        return values[0]
    pick = rng.random() * total
    running = 0.0
    for spec in values:
        running += max(0.0, spec.weight)
        if pick <= running:
            return spec
    return values[-1]
