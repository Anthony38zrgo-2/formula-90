# Suspension oscillation audit

## Scope and provenance

Source branch: `main-clean`.
HEAD: `f7fa7d46b559538a8dd80819f2f52e0dd7997573`.
The checkout contains pre-existing changes and the current candidate corrections.
Inventory: `scratch/race_driver_physical_pace_20261015/status_before_deep_suspension_audit.txt`.
This audit adds a diagnostic bench. It does not change suspension coefficients,
the production physical frequency, the integrator, or the artificial driver.

## Confirmed numerical defect

The coupled vehicle integrator evaluates velocity-dependent damping at the start
of a step, then advances velocity using that force explicitly. Its position
update is semi-implicit, but its damping update remains explicit. The damper,
vertical tire damping and progressive stop damping all enter this force update.

A positive damping coefficient is therefore insufficient to establish discrete
passivity. For an isolated linear damping mode with decay rate `lambda`, the
explicit velocity multiplier is `1 - lambda * duration`. Its magnitude must be
less than one for decay. A local directional linearization of the configured
vehicle reports a fastest estimated damping mode of approximately 261.48 per
second. At 120 Hz the corresponding multiplier is approximately -1.179. This
estimate is diagnostic: asymmetric damping, unilateral contacts and digressive
branches make the complete system nonlinear.

The independent unforced experiment below confirms the actual settling defect;
the conclusion does not depend solely on that linearization.

## Reproduction without the artificial driver

The bench uses the actual geometric F1 2030 configuration, a fixed planar ground,
stationary steering, zero tire tangential forces, zero drive/brake torques and a
small initial body roll velocity of 0.05 radians per second. It simulates three
seconds and measures peak velocities in the final second. There are no moving
ground surfaces or prescribed steering work. Event localization remains active.
Only the diagnostic integration step changes between rows.

| Diagnostic frequency | Final-second peak roll speed, rad/s | Final-second peak wheel travel speed, m/s | Maximum energy balance residual, J |
| --- | ---: | ---: | ---: |
| 120 Hz | 0.323385 | 0.495062 | 7235.776 |
| 240 Hz | 0.000000000204 | 0.000005422 | 5.214 |
| 480 Hz | 0.000000000194 | 0.000005102 | 1.079 |
| 960 Hz | 0.000000000189 | 0.000004961 | 0.254 |

Residual is `current mechanical energy + accumulated dissipation - external work
- steering work - initial mechanical energy`. The 120 Hz system keeps oscillating
and records 7398.42 J of dissipation while the finer cases settle with about
174-179 J. This is consistent with numerical replenishment of the energy that
the physical dampers remove. The fine-step convergence also rules out the
artificial driver as a necessary cause of this reproduction.

Command:

```powershell
$env:CARGO_TARGET_DIR='D:/Formula90s/scratch/physical_frequency_120_hertz_correction/cargo_target'
cargo test --release --manifest-path game/crates/vehicle-physics-engine/Cargo.toml --test suspension_oscillation_audit -- --ignored --nocapture
```

Evidence: `scratch/race_driver_physical_pace_20261015/suspension_oscillation_audit.log`.
The ignored diagnostic prints measurements; its successful execution is not a
passing stability acceptance gate.

## Other incomplete mechanisms

The world prescribes a rack velocity independently each force interval while
leaving prescribed rack acceleration at zero. Reversals can consequently change
prescribed component momentum at interval boundaries without an explicit
transition calculation. Existing kinetic-energy and steering-work checks do not
establish convergence for repeated reversals. This is a separate audit finding
requiring an isolated test; it is not proven to cause the unforced failure,
whose rack remains stationary.

Travel-stop projection removes instantaneous kinetic energy and checks that it
does not add energy. That does not certify the stability of the force advancement
between projections. Passing projection tests therefore does not cover the
unforced settling defect.

## Required correction and gates

The repair must remain stable at the required 120 Hz. Treat the coupled
velocity-dependent dissipation with a stable implicit or equivalent passive
update, including damper, vertical tire and stop terms and their unilateral or
digressive branches. Preserve the full mass coupling and evaluate actual
discrete dissipated work. Recheck stiffness and constraint interactions rather
than assuming damping alone establishes whole-vehicle stability.

Before resuming pace tuning, require unforced heave, roll and pitch decay;
repeated forced lateral reversals followed by decay; wheel unloading and stop
entry/exit; prescribed rack reversals and momentum/work accounting; and
coarse/fine energy convergence. A final Fuji run remains necessary after these
benches pass. These results do not prove that every runtime rollover has the
same cause, but they establish a reproducible suspension-integrator defect that
must be resolved first.
