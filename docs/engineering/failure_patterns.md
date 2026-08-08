# Failure Patterns (Failure Signatures)

This document catalogues recurrent failure signatures in the F1 2030 project to prevent agents from attempting invalid solutions.

If you encounter one of these signatures, refer to its recommended diagnostic step.

---

## SIGNATURE: `curb_vertical_oscillation`
**Scenario:** Car drives over a curb at high speed (e.g., `curb_test_medium_120`).
**Symptom:** Vehicle continues vertical oscillation after curb exit or suffers from extreme G-force spikes (-10G).
**Affected Subsystem:** Suspension / Track Geometry.
**Known Misleading Approaches:** 
- Increasing damping arbitrarily.
- Softening springs massively.
**True Root Cause / Check:** 
1. Check if `front_resting_ratio` and `rear_resting_ratio` have wildly different natural frequencies.
2. Check `rear_bump_stop_multiplier`. If it's `1.0`, the chassis is bottoming out on the track.
3. Check track geometry: 90-degree curb polygons act as rigid walls. Use chamfered slopes (e.g. `X = -4.4` instead of `-4.5` for the top vertex).
