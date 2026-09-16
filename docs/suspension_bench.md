# SUS-GEO-08 — Suspension bench report

Source: `game/crates/vehicle-physics-engine/examples/suspension_bench.rs`
(run: `cargo run -p vehicle_physics_engine --example suspension_bench --release`).
Fixture: healthy trailing-rocker test geometry (NOT the shipped car);
numbers below are mechanism evidence, not F1-2030 calibration.

## 1. Motion ratio r = ds/dq

| corner | r0 (rest) | min | max | shape |
|---|---|---|---|---|
| FL | 0.478 | 0.012 | 0.822 | rising-rate, then toggle collapse past q≈+0.08 |
| RL | 0.383 | 0.221 | 0.575 | falling-rate (digressive) across travel |

Consequences (SUS-GEO-04/09):
- Front bump travel MUST stay clear of the toggle (q≲+0.07 for this rocker);
  the travel envelope + `r<=0` guard enforce it, but calibration should place
  max bump on the rising slope, not past the peak.
- Rear is digressive everywhere: `F·dr/dq < 0` subtracts from the tangent
  rate (see §2). A rising-rate rear rocker is 09 design work.

## 2. Rest wheel rates (tangent k·r² + F·dr/dq, linear k·r², damper c·r²)

| corner | k tangent | k linear | geometric term | c bump | c rebound |
|---|---|---|---|---|---|
| FL | 35822 | 26705 | +34 % | 2693 | 3636 |
| RL | 16412 | 27118 | −39 % | 3005 | 4057 |

The F·dr/dq term is NOT negligible (±35–40 %): linear-rate models are wrong
by that margin on these rockers. Legacy-matched wheel rates hold by
construction (preload·r0 = static load), so static stance matches legacy
while the travel response is geometric.

## 3. Stops, reactions, balance

- Travel stops engage on wheel limits + damper stroke + mechanism clamp
  (SUS-GEO-04 tests: 120 mm slam engages, travel bounded, no NaN).
- Whole-wheel balance residual 0.00 N on the unit check; <2 % Fz settled
  in sim (SUS-GEO-05 tests). Load-path split at rest ≈ 99 % element /
  1 % wishbones (the spring carries body weight; wishbones carry
  lateral/moment loads) — registered per wheel, symmetric L/R.
- Anchor moments replace contact-patch moments in geometric mode; total
  force matches legacy within unsprung terms (axle totals agree).

## 4. Push/pull equivalence

Relabelling pushrod↔pullrod without touching geometry: max hub/damper
delta over the full sweep = **0.0 m exact**. No artificial grip by label.

## 5. Cost per tick (release, this machine)

- `solve_corner`: ~11 µs; `jacobian` (2 solves): ~22 µs.
- Suspension tick ≈ 6 solves/wheel → ~70 µs/wheel; anchor wrench +1 solve.
- 4 wheels ≈ 0.3–0.5 ms/tick at 120 Hz (~5 % of a frame budget).
- Debug builds are ~10–20× slower (PBD iterations unoptimized); CI timing
  uses small tick counts for that reason. No HOST/GPU cost (pure Rust).

## 6. Legacy vs geometric deltas (matched static stance)

- Static Fz identical (preload·r0 = legacy static load by calibration).
- Wheel rate at rest identical by construction; travel response differs:
  geometric adds progressivity (F·dr/dq), bump-steer/camber from linkage
  (SUS-GEO-06, replaces linear gains), anchor moments (jacking/anti-dive
  emerge, SUS-GEO-05), physical stops (SUS-GEO-04).
- No silent fallback anywhere: invalid geometry fails with wheel+field.

## Conclusions for SUS-GEO-09

1. Calibrate bump/droop INSIDE the rising slope (front q≲+0.07 on these
   arms; design arms for the travel actually used).
2. Prefer near-constant-r rockers (long arms, small angles) so the linear
   intuition holds and the F·dr/dq term stays a correction, not the rate.
3. Keep matched preload (equilibrium at design) + legacy ARB ratios as the
   starting calibration, then tune from telemetry (SUS-GEO-10 A/B).
