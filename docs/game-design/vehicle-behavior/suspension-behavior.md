# Suspension Behavior Specification

## Scenario: Straight-Line Braking
**Initial Conditions:** 
- Speed: 300 km/h
- Surface: Road
- Input: 100% Brake

**Expected Player-Visible Behavior:**
- Nose dives forward progressively.
- Rear lifts slightly.
- Car remains stable and in a straight line.

**Expected Telemetry:**
- Load transfer > 500 N to the front axle.
- Rear axle must NOT unload beyond 30% of its static weight.
- Pitch rate settles within 0.5s of full brake application.

**Pass Criteria:**
- Vehicle comes to a halt without spinning.
- Rear wheels do not lose contact with the road.

**Fail Criteria:**
- `rear_lift_off_g` is triggered (rear axle unloads completely).
- Chassis bottoms out on the track.
