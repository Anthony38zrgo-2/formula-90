"""Deterministic, regulation-informed dimensional profiles."""
from __future__ import annotations

from copy import deepcopy
from typing import Any


_PROFILES: tuple[dict[str, Any], ...] = ({
    "profile_id": "f1-2009-fw31",
    "label": "F1 2009 · Williams FW31 (3.100 m)",
    "vehicle_class": "formula_1_open_wheel",
    "basis": "2009 FIA envelope plus Williams FW31 launch dimensions",
    "targets": {
        "parameter-wheelbase": 3.100,
        "parameter-front-track": 1.450,
        "parameter-rear-track": 1.425,
        "parameter-front-tire-radius": 0.330,
        "parameter-rear-tire-radius": 0.330,
        "parameter-front-tire-width": 0.350,
        "parameter-rear-tire-width": 0.375,
    },
    "width_policies": {
        "front_tire_width": "centered",
        "rear_tire_width": "centered",
    },
    "constraints": {
        "maximum_overall_width_m": 1.800,
        "computed_front_overall_width_m": 1.800,
        "computed_rear_overall_width_m": 1.800,
        "wheel_bead_diameter_min_m": 0.328,
        "wheel_bead_diameter_max_m": 0.332,
        "complete_front_wheel_width_min_m": 0.305,
        "complete_front_wheel_width_max_m": 0.355,
        "complete_rear_wheel_width_min_m": 0.365,
        "complete_rear_wheel_width_max_m": 0.380,
    },
    "notes": [
        "Wheelbase and complete-wheel widths follow the Williams FW31 launch specification.",
        "Dry tire radius uses the 0.660 m maximum complete-wheel diameter in FIA article 12.4.2.",
        "Tracks are inferred from the 1.800 m FIA overall-width envelope and centered complete-wheel widths.",
        "Body width is preserved; 1.800 m is enforced as the complete-wheel envelope.",
    ],
},)


def dimension_profiles() -> list[dict[str, Any]]:
    """Return profiles without exposing mutable module state."""
    return deepcopy(list(_PROFILES))


def get_dimension_profile(profile_id: str) -> dict[str, Any]:
    for profile in _PROFILES:
        if profile["profile_id"] == profile_id:
            return deepcopy(profile)
    raise KeyError(profile_id)
