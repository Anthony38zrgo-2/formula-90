"""Scenario definitions — single responsibility: deterministic scenario fixtures.

Each factory returns a fully-interpolated `Scenario` (frames at 200 Hz) covering
the auditionable families for the v10_vehicle bank: idle->redline+shifts,
asphalt->sand->rumble (road/gravel/kerb), grass skid, and impacts.
Surfaces are limited to those present in the sample bank: asphalt (engine only,
no tire bed), sand, grass, rumble. No mixing or I/O here.
"""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass
class Frame:
    t: float
    rpm: float
    speed_kmh: float
    throttle: float
    gear: int
    slip: float
    surface: str


@dataclass
class Impact:
    t: float
    kind: str
    gain: float = 1.0


@dataclass
class Scenario:
    name: str
    duration_s: float
    frames: list[Frame] = field(default_factory=list)
    impacts: list[Impact] = field(default_factory=list)
    description: str = ""


def _lerp_frames(keyframes: list[Frame]) -> list[Frame]:
    if not keyframes:
        return []
    out: list[Frame] = []
    t = 0.0
    total = keyframes[-1].t
    idx = 0
    while t <= total + 1e-9:
        while idx + 1 < len(keyframes) and keyframes[idx + 1].t <= t:
            idx += 1
        if idx + 1 < len(keyframes):
            a, b = keyframes[idx], keyframes[idx + 1]
            span = max(1e-9, b.t - a.t)
            u = (t - a.t) / span
            f = Frame(t=t, rpm=a.rpm + (b.rpm - a.rpm) * u, speed_kmh=a.speed_kmh + (b.speed_kmh - a.speed_kmh) * u,
                      throttle=a.throttle + (b.throttle - a.throttle) * u, gear=b.gear if u > 0.5 else a.gear,
                      slip=a.slip + (b.slip - a.slip) * u, surface=b.surface if u > 0.5 else a.surface)
        else:
            k = keyframes[idx]
            f = Frame(t=t, rpm=k.rpm, speed_kmh=k.speed_kmh, throttle=k.throttle, gear=k.gear, slip=k.slip, surface=k.surface)
        out.append(f)
        t += 1 / 200.0
    return out


def make_idle_to_redline_with_shifts() -> Scenario:
    kfs = [Frame(0.0, 2200, 0, 0.15, 1, 0.0, "asphalt"), Frame(1.0, 6000, 35, 0.85, 1, 0.05, "asphalt"),
           Frame(1.05, 6200, 38, 0.0, 2, 0.0, "asphalt"), Frame(2.0, 9500, 85, 0.95, 2, 0.04, "asphalt"),
           Frame(2.05, 9700, 88, 0.0, 3, 0.0, "asphalt"), Frame(3.0, 12000, 135, 1.0, 3, 0.06, "asphalt"),
           Frame(3.05, 12200, 138, 0.0, 4, 0.0, "asphalt"), Frame(4.2, 14500, 195, 1.0, 4, 0.03, "asphalt")]
    return Scenario("idle_to_redline_with_shifts", 4.2, _lerp_frames(kfs), [], "Idle -> redline through 3 shifts, throttle lift on each shift (asphalt, engine only).")


def make_road_to_sand_with_kerb() -> Scenario:
    kfs = [Frame(0.0, 7000, 70, 0.7, 3, 0.03, "asphalt"), Frame(1.0, 7400, 74, 0.7, 3, 0.04, "asphalt"),
           Frame(1.4, 7500, 75, 0.7, 3, 0.05, "rumble"),   # kerb strike
           Frame(1.45, 7500, 74, 0.6, 3, 0.08, "sand"),    # off into sand
           Frame(3.0, 7100, 66, 0.65, 3, 0.14, "sand")]
    return Scenario("road_to_sand_with_kerb", 3.0, _lerp_frames(kfs), [], "Asphalt -> kerb rumble -> sand runoff.")


def make_grass_skid() -> Scenario:
    kfs = [Frame(0.0, 6500, 55, 0.8, 2, 0.05, "asphalt"), Frame(0.6, 6800, 50, 0.9, 2, 0.45, "grass"),
           Frame(1.5, 8200, 42, 1.0, 2, 0.62, "grass"), Frame(2.4, 7800, 35, 0.4, 2, 0.30, "grass")]
    return Scenario("grass_skid", 2.4, _lerp_frames(kfs), [], "High-slip grass skid with throttle.")


def make_impact() -> Scenario:
    kfs = [Frame(0.0, 6000, 80, 0.6, 3, 0.03, "asphalt"), Frame(1.0, 6200, 82, 0.6, 3, 0.03, "asphalt"),
           Frame(1.8, 6400, 84, 0.6, 3, 0.03, "asphalt"), Frame(2.8, 6600, 86, 0.6, 3, 0.03, "asphalt")]
    impacts = [Impact(1.0, "cone", 0.9), Impact(1.8, "barrier", 1.0), Impact(2.3, "hit_3", 0.7)]
    return Scenario("impact", 2.8, _lerp_frames(kfs), impacts, "Cone tap + heavy barrier hit + body hit.")


SCENARIO_FACTORIES = {
    "idle_to_redline_with_shifts": make_idle_to_redline_with_shifts,
    "road_to_sand_with_kerb": make_road_to_sand_with_kerb,
    "grass_skid": make_grass_skid,
    "impact": make_impact,
}
