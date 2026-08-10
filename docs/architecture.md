# Current runtime architecture

Status: **current architecture reference**. Verify runtime facts against
`PROJECT_STATE.md` and the loaded Godot scene before changing a subsystem.

## Authority boundaries

```text
GEVP GDScript                    C++20 GDExtension
vehicle.gd + wheel.gd            camera, visual presentation, audio, reset,
physical simulation              bootstrap and menu integration
```

GEVP is the sole authority for vehicle motion, wheel contacts, suspension,
transmission, and driving assists. Native presentation code reads vehicle state
through `VehicleAdapter`; it must not implement a second vehicle simulation.

## Gameplay composition

`GameBootstrap` starts a gameplay world through `WorldHudCompositor`:

```text
GameBootstrap
└── WorldHudCompositor
    ├── WorldViewport (fixed 640x360 3D world)
    │   └── WorldContent (track and vehicle scene)
    ├── WorldPresenter (ViewportTexture in the root canvas)
    └── HudLayer (unfiltered root-canvas HUD)
```

This is a deliberate rendering boundary: world-only postprocessing belongs on
`WorldPresenter`; HUD, minimap, and menus must remain outside that viewport.

## Vehicle composition

The active test scenes use a GDScript controller with a GEVP vehicle body:

```text
VehicleController (GEVP vehicle_controllergd.gd)
└── VehicleRigidBody (GEVP vehicle.gd on RigidBody3D)
    ├── RayCast3D wheels and collision shapes
    ├── ArcadeChaseCamera (C++ presentation consumer)
    ├── EngineAudioController (C++ presentation consumer)
    └── ResetMarker / ResetManager integration
```

The Jordan scene owns physical geometry and wheel layout. Visual assets are a
separate concern and must not be used as collision authority.

## UI and presentation status

- The active HUD is GDScript/Control-based: `arcade_race_hud.gd`,
  `arcade_speed_gauge.gd`, and `track_minimap_controller.gd` in `HudLayer`.
- `DrivingAidsController` emits HUD notifications; `TrackMapData` is
  presentation-only and is not lap-progress authority.
- `DirectionalVehicleSprite` remains registered and is used by the static
  directional validation scene. It is not the player rendering path.
- The unreferenced native `DebugHudController`, `StaticMinimapController`,
  `ResetManager`, and `DirectionalSpriteValidationController` were removed in
  REF-001 Phase 4 after a complete consumer audit. Reset behavior now requires
  a future, explicitly owned gameplay implementation rather than an unused
  native node.

## Documentation authority

Use these documents in this order:

1. `PROJECT_STATE.md` — current validated state and runtime invariants.
2. This document and `docs/architecture/` — current structural boundaries.
3. `docs/game-design/` — desired product direction.
4. `docs/decisions/` and files labelled historical — rationale and past cuts,
   not runtime authority.

Historical plans that mention `ArcadeCarController` as the active physics
engine describe earlier migration work. They must not be used to choose current
scene ownership or implementation targets.
