# race-intelligence-engine — Autonomous race intelligence

Scope: racing-line following for artificial drivers: track path loading,
Frenet projection, conservative speed profiling, local steering and
longitudinal control, automatic transmission supervision, and cadence
scheduling. Milestone 1 covers one solitary driver on the racing line;
multi-driver perception and racecraft arrive in later milestones.
Consumers: headless runners and the Godot bridge adapter.
Rules: physics authority stays in `vehicle-physics-engine`; this crate only
produces driver inputs in the `VehicleInput` contract. No rubber-banding, no
fictitious grip or power, no teleports. Determinism first: fixed step, seeded
randomness only.
Subfolders: `src/track/` owns path geometry, `src/local_driver/` owns
control and `src/scheduling/` owns cadence. The explicit binary entry point
`src/autonomous_driver_bridge.rs` owns loopback validation transport; it never
advances physics. The Godot harness owns the single authoritative world.

The active controller is `AutonomousDriver`, with geometric pursuit, bounded
steering rate, integral speed regulation and exclusive throttle/brake commands.
Its load-time tuning lives in `game/data/race_intelligence/solitary_driver_tuning.json`.
The interval-count scheduler preserves host fractions and exposes missed ticks;
the boolean compatibility wrapper reports only whether an update is due.
The historical Fuji tail is accepted only after validating the retained closed
loop and the known 819/989 waypoint layout. Unknown discontinuities are errors.

The active speed profile is derived from the required vehicle configuration,
including tire load sensitivity, aerodynamic axle loads, braking, and gearing.
The predictor owns explicit nominal grip reserves. Steering uses actual rack
geometry. Recovery uses side-specific vehicle clearance and continuous slew;
steering preview never advances the longitudinal braking target.
