# Coupled vehicle world: Godot candidate integration

## Delivery boundary

The preceding physics candidate was committed as `4c81c4dc`. Its backlog and validation record were committed as `bd34f28dc3d792bafb86fba1e6529f6eb8aba458` on `main-clean`.

This continuation implements an explicit Godot integration candidate. It does not activate the candidate by default, complete the 50-item backlog, or certify race performance. Production libraries and `game/BUILD_SOURCE` must be rebuilt and checked against HEAD after source publication.

## Physical ownership

`PhysicalVehicleWorldController` requires explicit vehicle, presentation-core, profile and physical-package bindings. Every vehicle has its own presentation core. It checks profile agreement and rejects unregistered moving Godot collision bodies.

At physics priority 1000, it queues each vehicle's current input at physical time plus the unconsumed host fraction, then advances the shared Rust world exactly once. Gear requests below -1 mean no change; accepted requests are consumed once. Tree pause pauses the world without consuming physical time.

Native vehicle proxies have zero collision layer/mask and gravity, use a custom integrator, and remain frozen. Their previous Godot force callback and the legacy core force callback return immediately under Rust ownership. A failed candidate stops scheduling and freezes its explicitly bound bodies.

## Presentation, telemetry and audio

An additive `f90_core_accept_physical_world_snapshot` export consumes the physical systems snapshot and reuses the existing CoreFrame, module and audio publication path. It does not integrate the shadow vehicle or advance a second clock. ABI 18 and its existing output layout are retained. Legacy stepping and direct shadow reset/refuel/tire/tuning mutations are blocked after physical ownership is established.

Snapshots include four solved suspension corners. Wheel and suspension renderers use the Rust hubs, bases, rocker angle, joint endpoints and spin angles. Only rigid visual transforms are derived in GDScript. Cosmetic front packaging and the second linkage solver are bypassed in this mode. Native surface, normal-load, spin, thermal, fuel and powertrain consumers receive the physical state.

Collision events are published as a signal and may trigger the existing impact samples for both affected vehicles, with the existing 0.2-second cooldown. Continuous scraping and perceptual audio acceptance remain separate work.

## Services and lifecycle

Native vehicle reset, refuel and tire-replacement requests are routed into the Rust world and immediately republish snapshots. Reset accepts a complete finite position/yaw pair and synchronizes the presentation clock. Vehicle removal releases its world entity and clears the presentation core's cached vehicle pointer. World-library closure destroys owned worlds before unloading.

The profile JSON is transported unchanged when creating the world and registering vehicles. Godot's numeric JSON conversion is normalized for integral values, and native document serialization uses full precision. This preserves strict Rust integer fields without relaxing their validation or rounding physical configuration through GDScript.

The world library exports a full source commit identifier. Its loader requires agreement with the native source identifier and `BUILD_SOURCE`. Old libraries without this capability are rejected.

## Explicit activation

Both original and MP4/6-inspired F1 2030 session scenes contain an inactive controller with explicit player/core bindings. With a source-matched production runtime, the canonical entrypoint supports:

```powershell
./scripts/run_f1_94.ps1 -PhysicalWorldPackage "D:/Formula90s/scratch/coupled_vehicle_complete_implementation/candidate_review_without_debug_symbols/fuji_physical_world.json"
```

The package remains a diagnostic artifact whose source and geometry digests are verified by Rust. Existing BUILD/HEAD checks still run. Validate the installed production libraries after each source commit before using this command.

## Validation evidence

Evidence is under `scratch/coupled_vehicle_godot_integration/`:

- Three Rust presentation tests: physical ownership and clock isolation; rejection of incompatible identity/time/mass; finite, nonzero sampler output from physical snapshots.
- One existing Rust pit-service facade regression test.
- Twenty-five coupled world tests.
- Godot controller contract with two mock vehicles: input order, one advance, retained host fraction, pause, services and removal.
- Native Godot harness with two real native vehicle/core pairs and the source-verified Fuji collision package: registration, snapshots, stepping, physically inert Godot proxies, pose agreement, 20 kg refuel and 718 kg operating mass, tire replacement, requested-pose reset, pause and removal.
- The same native harness instantiates the real wheel/link renderers, checks Rust/rendered hub and damper agreement, and verifies zero second linkage solves.
- Source-parity negative test: reject an incorrect diagnostic BUILD_SOURCE, restore it, then successfully reload the matching library.
- A fresh reduced Godot C++ SDK build and an isolated extension containing the production world-interface, vehicle and core translation units. No SDK object/library caches were copied into the fresh build.

The harness runs in an isolated diagnostic project with audio playback disabled. The Rust sampler test checks signal generation, not a listening judgment. These checks do not replace a full production extension rebuild, canonical scene run, sustained driving, impact matrix, rendered whole-car review, replay parity or full-grid performance acceptance. The previously recorded performance failure remains open.
