"""Bank spec — declarative mapping of source and synthesized samples to bank roles.

The bank is derived from the original F1-1998 engine internal + Grand Prix sample
set in `assets-lowpoly-python/sounds/`. Entries either map a source filename to
a stable bank key or name a deterministic synthesis recipe. This is the single
place to change what the bank contains.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

DEFAULT_SOURCE_DIR = Path("assets-lowpoly-python/sounds")


@dataclass(frozen=True)
class SpecEntry:
    key: str            # stable bank key / output filename stem
    source: str | None  # filename relative to SOURCE_DIR; None for synthesis
    role: str           # semantic role, e.g. engine_idle, surf_grass, impact_hit
    loop: bool
    category: str       # engine | shift | surface | impact | start
    native_rpm: float | None = None  # for engine bands: RPM at which sample was recorded
    synthesis: str | None = None     # deterministic synthesis recipe


# Native RPM per engine band (measured via full-loop dominant spectral peak as
# firing_freq*12; V10 4-stroke firing = RPM/12). Values are the honest recorded
# revs, NOT a monotonic ladder — each band is pitch-corrected to the current RPM,
# so non-monotonicity is fine. redline = stable high segment (t=1-2s) of the 5s file.
ENGINE_BAND_NATIVE_RPM: dict[str, float] = {
    "engine_idle": 3941.0,   # 98_int_idle.wav  dominant 328.4 Hz
    "engine_low": 7429.0,    # 98_int_low.wav   619.1 Hz
    "engine_mid": 8196.0,    # 98_int_med.wav   683.0 Hz
    "engine_high": 5580.0,   # 98_int_high_1.wav 465.0 Hz
    "engine_redline": 7687.0,  # 98_int_max_5.wav stable segment 7655-7687 Hz
}


# Mapping source -> bank key. Chosen variants prefer the higher sample-rate or
# longest representative source where multiple exist (e.g. _2 at 44.1 kHz).
BANK_SPEC: tuple[SpecEntry, ...] = (
    # --- Engine loops ---
    SpecEntry("engine_idle", "98_int_idle.wav", "engine_idle", True, "engine"),
    SpecEntry("engine_low", "98_int_low.wav", "engine_low", True, "engine"),
    SpecEntry("engine_mid", "98_int_med.wav", "engine_mid", True, "engine"),
    SpecEntry("engine_high", "98_int_high_1.wav", "engine_high", True, "engine"),
    SpecEntry("engine_redline", "98_int_max_5.wav", "engine_redline", True, "engine"),
    SpecEntry("engine_tc", "98_INT_tc.wav", "engine_tc", True, "engine"),
    # --- Shifts / start (one-shots) ---
    SpecEntry("shift_up", "98_int_shift.wav", "shift_up", False, "shift"),
    SpecEntry("shift_down", "98_int_shift_2.wav", "shift_down", False, "shift"),
    SpecEntry("shift_3", "98_int_shift_3.wav", "shift_3", False, "shift"),
    SpecEntry("engine_limiter", "98_INT_limiter_engage_disengage.wav", "engine_limiter", False, "engine"),
    SpecEntry("engine_start_backfire", "98_start_backfire.wav", "engine_start_backfire", False, "start"),
    SpecEntry("int_backfire", "ALT_int_backfire.wav", "engine_backfire", False, "engine"),
    SpecEntry("int_backfire_2", "ALT_int_backfire_2.wav", "engine_backfire", False, "engine"),
    SpecEntry("engine_starter", "98_starter.wav", "engine_starter", False, "start"),
    # --- Surfaces (loops) ---
    SpecEntry("surf_grass", "GP_grass_2.wav", "surf_grass", True, "surface"),
    SpecEntry("surf_sand", "GP_sand_2.wav", "surf_sand", True, "surface"),
    SpecEntry("surf_rumble", "GP_rumble_2.wav", "surf_rumble", True, "surface"),
    # --- Impacts (one-shots) ---
    SpecEntry("impact_hit_1", "GP_hit1.WAV", "impact_hit", False, "impact"),
    SpecEntry("impact_hit_2", "GP_hit2.WAV", "impact_hit", False, "impact"),
    SpecEntry("impact_hit_3", "GP_hit3.WAV", "impact_hit", False, "impact"),
    SpecEntry("impact_hit_4", "GP_hit4.WAV", "impact_hit", False, "impact"),
    SpecEntry("impact_barrier", "GP_barrier.wav", "impact_barrier", False, "impact"),
    SpecEntry("impact_cone", "GP_conehit.wav", "impact_cone", False, "impact"),
    SpecEntry("impact_fire", "GP_fire.WAV", "impact_fire", False, "impact"),
    SpecEntry("impact_scrape", None, "impact_scrape", False, "impact", synthesis="flat_floor_scrape_v2"),
)

SPEC_BY_KEY: dict[str, SpecEntry] = {e.key: e for e in BANK_SPEC}


def spec_for_key(key: str) -> SpecEntry | None:
    return SPEC_BY_KEY.get(key)


def roles_by_category(category: str) -> list[str]:
    return [e.role for e in BANK_SPEC if e.category == category]
