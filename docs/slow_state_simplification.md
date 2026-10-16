# P4: simplified slow states

Implemented on main-clean above source base 5166795a6bd116fc1ed7b3f091c624433c815c67. This atomic P4 publication includes implementation, tests, benchmark runner and evidence documents. Validation was performed before publication with source hashes; the new commit requires a native rebuild before launching again.

## Audit and implementation

The reference backend remains selectable independently of tire, suspension and aerodynamics backends. The canonical launcher defaults all four to simplified. Direct Rust construction and old serialized states retain standard for compatibility.

`tire_thermals.rs`: three tread zones, carcass and gas have distinct contact/camber, hysteresis and pressure consequences. No physical temperature node was safely redundant. Simplified evolves these at 20 Hz, accumulates zonal heat and contact/airflow integrals at every mechanical step, and computes simultaneous conservative internal exchange. Gas temperature is held between updates, while compression pressure is corrected every mechanical step: holding pressure itself would alter fast suspension/tire feedback.

`brake_thermals.rs`: rotor and rim inertia are distinct and retained at 10 Hz. Mechanical braking work and diagnostics are recorded every tick. Rim-to-tire transfer has matched debit/credit accumulators. Duct flow/drag remains instantaneous, evaluated once per axle instead of twice; averaged conductance feeds the thermal update.

`powertrain_thermals.rs`: engine block, water and oil retain separate capacities and exchange/cooling paths, evaluated at 10 Hz. Instantaneous torque times angular speed is integrated before averaging. Cooling drag remains fast; overheat power derating updates with temperature.

`tire_wear.rs`: existing wear law remains in the mechanical path. Work, surface effects and wear accumulate every physical interval. No low-rate averaging of wear or irreversible work.

`simulation.rs`: independent serialized clocks and energy accumulators dispatch the candidate. Variable intervals are partitioned at domain boundaries without discarded time; tire service flushes the old set, backend switching flushes all pending work, and partial windows survive serialization. Coupled-world authoritative brake energy and damping-squared carcass heat remain supported. Existing canonical coupled-world restrictions for the other simplified backends remain.

Configuration: `game/data/config/slow_state_updates.json` provides experimental 20/10/10 Hz rates, validated as finite and within (0,120]. Mechanics stays 120 Hz. There is no physical tuning or deletion of thermal consequences. Reduced redundant state consists of derived cooling/transport accumulators and cached mechanical modifiers, rather than merged temperature nodes.

## Automated evidence

16 candidate integration tests pass, including exact update counts, conservation, anti-correlated work inputs, irregular/long intervals, serialization, backend switch, tire service, fast pressure/drag, wear, fade and overheating. 55 relevant existing integration tests pass. The two-minute prescribed comparison has maxima: tire gas 0.095 C, pressure 0.0581 kPa, rotor 3.177 C, water 0.449 C and oil 0.277 C.

The broader suite is not fully green: 162 library tests pass with the unchanged aero wheel-reference fixture failing; the pre-change fuel suite has 7 passes and a 600 versus 597.795 litre fixture failure. These unrelated profile mismatches were preserved.

## Controlled 22-car comparison

Evidence: `scratch/slow_state_grid_22_120_hertz_20261016_100512`, five alternating pairs, 3 s warmup and 12 s measured per run, one pinned thread, 120 Hz. Other three backends held simplified. Source/executable hashes and runner are preserved. Checksums are deterministic within each backend; outputs are finite. Every candidate vehicle records 1800 mechanical intervals, 300 tire, 150 brake and 150 powertrain updates over 15 s.

| Workload | Standard grid microseconds | Simplified grid microseconds | Paired mean reduction | 95% interval |
| --- | ---: | ---: | ---: | --- |
| Isolated slow states | 27.142 | 17.786 | 33.840% | 25.735 to 41.945% |
| Prescribed mechanical replay | 197.523 | 212.806 | -7.729% | -40.876 to 25.418% |
| Free flat-road dynamics | 213.233 | 221.411 | -4.109% | -22.153 to 13.935% |

Only the isolated thermal gain is demonstrated. Whole-simulation means are slower in this sample and intervals include zero. No global speed or game FPS improvement is established. Prescribed work comparison peaks at 6.323 C rotor and 0.603 C water difference; free dynamics differs up to 1.970 m/s, so strict trajectory parity is not claimed. No Fuji, collisions, racecraft, Godot raycast, audio, rendering or human handling acceptance is inferred from the Rust benchmark.

## Runtime

Canonical usage: `./scripts/run_f1_94.ps1`; reference slow states: `./scripts/run_f1_94.ps1 -SlowStateBackend standard`. `-SmokeSlowStates` exercises startup and half a second of physics. Runtime rebuild/smoke evidence is recorded separately under `scratch/slow_state_simplification`. Godot MCP validated the smoke script; no live bridge was installed, so runtime logs do not establish visual or driving acceptance.

Full clean native rebuild completed. Fifteen installed optimized Rust copies and three DSP copies match fresh build products; BUILD/HEAD parity and final dirty-source hashes pass. Four canonical scenarios passed: default simplified, standard slow states with simplified mechanics, simplified slow states with standard mechanics, and full Fuji spawn/surfaces/camera/HUD/fuel/lap-timing smoke. No script/runtime errors occurred in successful logs. First clean import crashed while importing fonts (-1073741819); a preserved canonical retry completed successfully. Evidence: `scratch/slow_state_simplification/rebuild_report.json`. Human driving review remains pending.
