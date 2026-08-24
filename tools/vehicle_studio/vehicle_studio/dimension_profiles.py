"""Deterministic, regulation-informed dimensional profiles."""
from __future__ import annotations

from copy import deepcopy
from typing import Any


_PROFILES: tuple[dict[str, Any], ...] = ({
    "profile_id": "f1-2021-nominal",
    "label": "F1 2021 nominal (3.640 m)",
    "vehicle_class": "formula_1_open_wheel",
    "basis": "User-supplied F1 2021 dimensional envelope",
    "targets": {
        "parameter-wheelbase": 3.640,
        "parameter-front-track": 1.635,
        "parameter-rear-track": 1.575,
        "parameter-front-tire-radius": 0.335,
        "parameter-rear-tire-radius": 0.335,
        "parameter-front-tire-width": 0.305,
        "parameter-rear-tire-width": 0.405,
    },
    "width_policies": {
        "front_tire_width": "centered",
        "rear_tire_width": "centered",
    },
    "constraints": {
        "maximum_overall_width_m": 2.000,
        "computed_front_overall_width_m": 1.940,
        "computed_rear_overall_width_m": 1.980,
        "nominal_rim_diameter_m": 0.3302,
        "fia_rim_lip_outer_diameter_m": 0.358,
        "front_rim_mounting_width_m": 0.3480,
        "rear_rim_mounting_width_m": 0.4293,
    },
    "notes": [
        "Wheelbase is the midpoint of the supplied 3.550-3.730 m typical range.",
        "Tire radius is derived from the supplied 0.670 m nominal diameter.",
        "Rim mounting widths are validation metadata, not tire mesh widths.",
        "Body width is preserved; 2.000 m is enforced as an envelope, not a scale target.",
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

