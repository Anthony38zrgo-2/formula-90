# Live tire capacity for autonomous driving

The candidate reads the tires belonging to the authoritative `F90Core` entity in the canonical Godot session. It does not read the inactive legacy vehicle simulator when the core owns force evaluation. The additive snapshot entry points retain the existing frame layouts and reject unsupported physical-world ownership.

The tire solver retains the longitudinal and lateral force capacities it already calculated. Reading them neither advances physics nor evaluates another tire model. Four-wheel loads and capacities remain separate until axle aggregation, preserving load sensitivity and lateral load transfer. The current downforce is already represented in measured tire loads and is not added a second time.

## JSON configuration

Both driver tuning documents contain a required `live_grip_control` object. All operating frequencies, observation age limits, planning horizon, capacity correction limits, slew rates, saturation thresholds and steering guard settings are loaded from JSON. There are no silent tuning defaults. Invalid schedules and stale observations are rejected.

| Work | Initial frequency | Inputs |
| --- | --- | --- |
| Control and fast intervention | 60 Hz | Pose, speed, yaw response, steering, wheel loads, utilization, demand and slip relative to peak |
| Capacity estimation | 20 Hz | Actual lateral/longitudinal capacities and delivered forces per wheel |
| Local speed planning | 10 Hz | Cached axle capacity corrections and forward racing-line geometry |
| Slow state | 5 Hz | Operating mass, axle weight distribution, effective grip diagnostics and brake efficiency |

The harness omits capacity and slow packets between their scheduled updates. The controller validates those packets only on arrival. Held values have separate age limits. A sudden contact loss or a saturated front axle with insufficient yaw response is evaluated every control update, independently of the planner.

The baseline remains selectable with `live_grip_control.enabled = false`. It retains the original static performance profile. The candidate uses the nominal profile as its initial reference and corrects each axle against observed force capacity. Corrections preserve the speed-dependent aerodynamic reference, are bounded and recover progressively. They are estimates for upcoming track conditions, not guarantees of future grip.

The local planner revisits only a forward window rather than recomputing the complete circuit. Longitudinal control retains pedal slew limits and exclusive throttle/brake commands. A friction-ellipse estimate bounds brake application while cornering. Detected front saturation bounds additional steering demand instead of indefinitely increasing steering angle.

## Running and evidence

`scripts/run_test_ai.ps1` snapshots the selected JSON into a new evidence directory and uses the canonical launcher, including BUILD/HEAD validation. `-DriverTuningPath` selects an experiment and `-Headless` runs the two-lap validation without a window. Default invocation opens the observation window.

`live_grip_control.record_observations` enables JSON-lines recording for replay diagnostics. It is disabled in the normal configuration. Telemetry records axle corrections, peak utilization, saturation decisions and actual update counters.

`benchmark_live_driver_capacity` replays recorded canonical observations through a specified number of independent controllers. It compares the static reference, the multirate candidate and a candidate doing estimation/planning at control frequency. This measures controller work only: it excludes physics, rendering, transport and snapshot production, and therefore is not a full-grid frame-time benchmark.

Validation evidence for this implementation belongs in `scratch/ai_live_grip_20261016/`. Automated results and remaining limitations are recorded after the runtime comparison; implementation alone does not establish driving acceptance.

## Validation on 2026-10-16

All three canonical Fuji trials completed two laps without footprint violations or runtime script errors. Utilization remained 0.95, nominal grip 0.65, physics 120 Hz, automatic transmission enabled and driving aids mask 36. The four physics backends remained simplified.

| Driver variant | Flying lap | Maximum lateral path error | Braking control samples |
| --- | --- | --- | --- |
| Static reference | 68.500 s | 2.183 m | 1198 / 8548 |
| Live capacity, 60/20/10/5 Hz | 66.133 s | 2.602 m | 654 / 8303 |
| Live capacity, all updates at 60 Hz | 66.750 s | 2.498 m | 704 / 8325 |

The multirate candidate was 2.367 s faster than the static reference in this trial. Between circuit distances 3000 and 3300 m it held 100% throttle in both laps, without braking. The static reference reduced throttle to 15.6% within that interval. This is one flying lap per variant, not a sustained pace or strict behavioral-equivalence certification.

The final isolated replay of 8303 control batches through 22 controllers measured 0.284 ms mean / 1.927 ms 95th percentile for multirate scheduling, versus 1.612 ms mean / 2.072 ms 95th percentile for all updates at 60 Hz. The static reference cost 0.016 ms mean. Multirate reduces mean controller work by approximately 82.4% relative to recalculating all live-capacity work at 60 Hz, while adding work relative to the old static controller. All replay controllers deliberately share their planning phase; the multirate maximum batch was 2.377 ms. These numbers exclude snapshot production, JSON/TCP transport, physics and rendering.

The implementation passed 32 targeted controller tests, the authoritative snapshot/ABI test and 10 tire simplification regression tests. Debug and Release native builds and both DSP ABI test configurations passed. The canonical runtime loads the refreshed Release physics/core libraries; source and installed hashes are retained with the evidence. Pre-existing dirty source and generated binaries were inventoried and preserved; no source commit was created.

Full-grid end-to-end frame time, distributed fleet scheduling, longer runs and human driving review remain separate acceptance gates. The normal JSON retains utilization 0.80; utilization 0.95 is selected in the experiment JSON rather than changing global risk during this test.
