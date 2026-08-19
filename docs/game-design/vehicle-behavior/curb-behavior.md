# Curb Behavior Specification

## Scenario: curb_test_medium_120
**Initial Conditions:**
- Speed: 120 km/h
- Surface: Road to Curb transition (5cm vertical step, chamfered)
- Input: 0% steering, constant throttle

**Expected Player-Visible Behavior:**
- Wheel rises upon impact.
- Chassis reacts moderately but does not catapult.
- Tire maintains or rapidly recovers track contact.
- One rebound is acceptable.
- Vehicle trajectory remains controllable.
- Chassis must NOT collide with the track surface.

**Expected Telemetry:**
- `suspension_travel` remains > 0 (no bottoming out).
- Peak vertical acceleration < 3.0G.
- Wheel contact loss < 100ms.
- Settling time < 0.5s.

**Pass Criteria:**
- No visual or physical catapulting.
- Car maintains forward vector.

**Fail Criteria:**
- Chassis impact (Infinite G-force / -10G spike).
- Car rolls over.
- Endless vertical oscillation.
