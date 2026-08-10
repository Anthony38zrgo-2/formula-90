# REF-001 Phase 5 — vehicle state boundary

## Consumer evidence

| Consumer | Required vehicle reads |
| --- | --- |
| `ArcadeChaseCamera` | global transform, linear velocity, steering amount, speed |
| `EngineAudioController` | RPM, throttle, speed, current gear |
| `VehicleVisual3DController` | global transform/position, linear velocity, steering input |
| `DirectionalVehicleSprite` | global transform and linear velocity when a vehicle path is supplied |

No active native consumer mutates vehicle transform, linear velocity, angular
velocity, or other GEVP state. The former sole mutation consumer,
`ResetManager`, was removed in Phase 4 because it was not instantiated.

## Change

- Replaced `VehicleAdapter` with `VehicleStateReader`.
- The new boundary exposes only the read operations proven above.
- Removed node escape and all vehicle mutators from the native presentation
  boundary.
- Updated camera, audio, visual, and directional-sprite consumers.
- No command adapter was introduced: adding one without a current mutation
  consumer would create speculative architecture.

## Validation

- Repository search found no `VehicleAdapter` reference or vehicle-command
  mutator in native runtime sources. Remaining legacy-plan text is historical.
- `scripts/build_windows.ps1` passed.
- Headless loads exited successfully for `test_field.tscn`, `player_car.tscn`,
  and `static_directional_car.tscn`.
- The smoke output still contains known, unrelated baseline issues: missing
  `EngineAudioConfig` in `test_field` and missing legacy `jordan_191` visual
  assets when loading `player_car.tscn`.

## Follow-up

When gameplay reset/spawn behavior is implemented, define its authority first
and introduce a narrowly scoped command boundary only for the required command.
