# Coupled world race-cost benchmark

## Source and scope

- Branch: `main-clean`
- Source base: `adcc109f8db03d27f02950c828ac31d99c9c69c1` (items 17 and 28–35)
- Item 46 benchmark and optimizations were measured on top of that commit in the
  working tree.
- `game/BUILD_SOURCE`: `ce0f153c70c6b888964bea428ded5c300c5fe5ab` (not resealed)
- Hardware: AMD64 Family 25 Model 80, 12 logical processors, Windows x86_64.
- Profile: `game/data/vehicles/f1_2030/f1_2030_v10_geometric.json`
- Physical package: `scratch/coupled_vehicle_complete_implementation/candidate_review_without_debug_symbols/fuji_physical_world.json`
- Track metadata: `game/tracks/fuji76_77/metadata/track.json` (50 authored grid spots)
- Benchmark: `coupled_vehicle_race_cost` release example.

The benchmark registers the first N authored grid spots, advances a 120 Hz world
for 120 warmup steps, then records 20 measured `advance_host_interval(1/120)` wall
times. It excludes rendering, transport, audio and presentation. `idle` leaves the
settled cars at rest; `rolling` applies a constant 30 m/s forward speed to the pack
after warmup; `mixed` and `mixed-rolling` register the first car as a 120 Hz player
and the rest as 60 Hz AI. This is a world-step cost, not a scripted race replay.

## Results

Step interval at 120 Hz: `8.333 ms`.

| Scenario | Mean ms | p50 ms | p95 ms | p99 ms | Mean / interval | p95 / interval |
|---|---:|---:|---:|---:|---:|---:|
| 1 car, idle | 1.879 | 1.797 | 3.318 | 3.318 | 0.23 | 0.40 |
| 20 cars, idle | 38.977 | 36.957 | 62.886 | 62.886 | 4.68 | 7.55 |
| 20 cars, rolling | 48.850 | 46.865 | 70.378 | 70.378 | 5.86 | 8.45 |
| 50 cars, idle | 98.799 | 94.280 | 158.581 | 158.581 | 11.86 | 19.03 |
| 50 cars, rolling | 129.057 | 124.591 | 189.095 | 189.095 | 15.49 | 22.69 |

The provisional M3 proposal (20 cars: mean ≤30%, p95 ≤50%; 50 cars: p95 ≤60%)
fails in every fleet scenario by roughly one order of magnitude. A repeated 20-car
idle sample measured 40.0 ms mean and 65.1 ms p95; single-sample variation is a few
percent and does not change the conclusion.

## Mixed-frequency candidate

The candidate mixed policy is:

- player vehicles at the world frequency, 120 Hz;
- all AI vehicles at 60 Hz, updated together every second world tick;
- on an AI update tick the AI phase runs before the player phase; AI state is then
  held for the following player tick;
- AI state held between updates;
- player-AI contacts solved at 120 Hz with the AI treated as fixed (the player
  receives the response; the AI velocity does not change in that phase);
- AI-AI and AI-object contacts solved only on the 60 Hz AI phase;
- AI uses a 1/60 s maximum integration substep;
- dynamic objects remain at 120 Hz.

The benchmark modes `mixed` and `mixed-rolling` register the first car as the
120 Hz player and the rest as 60 Hz AI.

| Mixed scenario | Player frame mean | Player p95 | AI phase mean | AI p95 |
|---|---:|---:|---:|---:|
| 20 cars mixed | 36.544 ms | 73.941 ms | 38.842 ms | 53.176 ms |
| 20 cars mixed-rolling | 38.764 ms | 74.419 ms | 41.322 ms | 54.324 ms |
| 50 cars mixed | 95.489 ms | 184.656 ms | 103.194 ms | 137.338 ms |

The split budget uses the same fractions as the uniform proposal: player frame
mean ≤30%/p95 ≤50% (20 cars) or p95 ≤60% (50 cars); AI phase mean ≤30%/p95 ≤50%
of the 16.67 ms AI interval (20 cars) or p95 ≤60% (50 cars). Every mixed case fails
the player and AI budgets.

Mixed frequency improves the mean only modestly in the idle pack (20 cars 36.5 ms
versus 39.0 ms uniform) and worsens p95 because all AI update together. The AI
phase itself costs 38.8 ms for 20 cars and 103.2 ms for 50, well above its 16.67 ms
interval.

## Stage profile

Averaged over the measured steps:

| Scenario | Vehicle forces ms | Shape attachment ms | CCD ms | Contact generation ms | Contact solve ms | Profile total ms | Measured mean ms |
|---|---:|---:|---:|---:|---:|---:|---:|
| 20 idle | 23.0 | 1.81 | 0.064 | 0.093 | 0.0 | 32.0 | 39.0 |
| 20 rolling | 22.9 | 1.81 | 10.1 | 0.094 | 0.0 | 41.7 | 48.9 |
| 50 idle | 58.1 | 4.55 | 0.204 | 0.333 | 0.0 | 80.9 | 98.8 |
| 50 rolling | 57.4 | 4.50 | 29.9 | 0.339 | 0.0 | 110.6 | 129.1 |

The difference between profile total and measured mean is the transactional world
clone and host-interval bookkeeping. The contact solver is inert in these packs
because the cars are supported by compliant tire rays, not rigid chassis contacts.

## Implemented optimizations

- Terrain AABBs are computed once when the world is created instead of once per
  vehicle shape and query.
- Attached shapes cache their world AABB, avoiding repeated `compute_aabb` calls in
  contact generation and CCD.
- The world computes the post-advance attached shapes once per physical step and
  reuses them for CCD and contact generation. A localized impact recomputes only
  the event shapes.
- `WorldStepProfile` records force, attachment, CCD, contact generation, solve,
  correction and finalize time for reproducible bottleneck evidence.

The 20-car idle mean was `52.19 ms` before these changes and `50.64 ms` after the
same short two-step warmup: about 3 percent. Geometry and shape work is not the
dominant cost.

## Finding

The per-car force step is about `1.15–1.18 ms` at rest, and the isolated kernel
profile records about `0.455 ms` for one complete coupled evaluation. The world
performs more than one evaluation per vehicle per step (support, integration and
sometimes braking), so 20 cars have a model-evaluation floor around `18 ms` before
contacts, shape attachment and the transactional clone. The 20-car mean target is
`2.5 ms`; the 50-car p95 target is `5 ms`.

Geometry, broadphase and matrix-solver optimization cannot close this gap without
changing the per-vehicle model or the execution architecture. The realistic options
are:

1. approve a simpler production vehicle model for the world path;
2. parallelize independent per-vehicle force evaluations in a deterministic order;
3. reduce evaluations per step and transactional clone cost;
4. revise the M3 budget to the measured geometric-profile cost.

The mixed-frequency policy reduces the mean only modestly and the all-AI-together
phase creates a large p95 burst. Staggering AI groups across ticks would spread the
cost, but the selected policy is all AI together; meeting the split budget still
requires model simplification, parallelism, fewer evaluations or a revised budget.

No physics, tuning profile or gate threshold was changed by this benchmark.

## Limits

- The mixed policy holds AI between updates and treats AI as fixed in player-AI
  contacts; AI does not receive player impulse until its next AI phase. AI uses a
  1/60 s maximum integration substep, so its trajectory differs from the 120 Hz
  player. Inputs are applied at 120 Hz tick boundaries; AI inputs apply on the next
  AI phase.
- Short diagnostics, not a full race replay; no scripted driver inputs, pit stops,
  collisions over a full lap or grid-wide pileup.
- The idle/rolling pack does not exercise rigid multi-car contact; item 43 covers
  selected impacts, not race-cost percentiles under contact.
- The benchmark excludes rendering, audio, transport, native serialization and
  presentation.
- Hardware-specific result; the provisional gate remains pending human approval.
- `game/BUILD_SOURCE` does not match HEAD; no canonical runtime was launched.
