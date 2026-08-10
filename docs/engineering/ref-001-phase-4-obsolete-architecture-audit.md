# REF-001 Phase 4 — obsolete native architecture audit

## Audit scope

The audit traced candidate classes through Godot scenes, scripts, resources,
tests, native includes, registration, SCons source discovery, documentation,
and Git history.

## Classification

| Component | Result | Evidence |
| --- | --- | --- |
| `DirectionalVehicleSprite` | Retained | Instanced by `game/scenes/vehicles/static_directional_car.tscn`; its metadata and atlas remain active validation assets. |
| `ArcadeChaseCamera` | Retained | Instanced by `test_field.tscn` and `player_car.tscn`. |
| `EngineAudioController` | Retained | Instanced by `test_field.tscn` and `player_car.tscn`. |
| `VehicleVisual3DController` and config | Deferred | No active scene instance, but typed `.tres` resources remain. Removing it would require a separate resource/asset migration. |
| `ArcadeCarController` | Rejected as removal target | No implementation is present; remaining mentions are historical documentation. |
| `DebugHudController` | Removed | Only its own source/header, GDExtension registration, and documentation mentioned it. |
| `StaticMinimapController` | Removed | Only its own source/header, GDExtension registration, and documentation mentioned it. |
| `ResetManager` | Removed | Only its own source/header, GDExtension registration, and historical documentation mentioned it. |
| `DirectionalSpriteValidationController` | Removed | Only its own source/header and GDExtension registration mentioned it; its referenced validation scene no longer existed. |

## Changes

- Removed the four confirmed unreferenced native classes and their
  `GDREGISTER_CLASS` entries.
- Removed the nonexistent directional-validation scene from Windows and Linux
  smoke-test lists.
- Kept the active static directional sprite path intact.

## Validation

- `scripts/build_windows.ps1` passed and linked the GDExtension.
- Headless scene loads passed for `main_menu.tscn`, `debug_hud.tscn`, and
  `static_directional_car.tscn`.
- A repository search found no runtime references to the removed class names.

## Follow-up

The active reset behavior has no current owner after removing the unused native
node. A future gameplay task must define reset authority and spawn semantics
before adding a replacement; this refactor deliberately does not invent one.
