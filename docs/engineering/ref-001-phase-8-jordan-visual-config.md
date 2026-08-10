# REF-001 Phase 8 — Jordan typed visual configuration

## Scope

Connect the existing typed vehicle-presentation resource path to the canonical
Jordan 1995 vehicle without changing GEVP configuration, collision, wheel
geometry, or generated assets.

## Change

- Added `jordan_1995_visual_3d.tres` as the explicit presentation contract for
  the canonical Jordan scene.
- Added `VehicleVisual3DController` as a presentation-only child of
  `Jordan1995Vehicle` and made `ChassisVisual` its child so the controller's
  top-level presentation pose can affect only chassis visuals.
- Configured the four wheel paths to their `Orientation` children, never their
  GEVP `Pivot` nodes.
- Set extra visual wheel spin and steering to zero. GEVP already owns those
  transforms on the pivots, so a second animation layer would duplicate motion.
- Kept the optional surface-probe path. It becomes active only in compositions
  that provide `SurfaceProbes` beside the vehicle.
- Generalized stale V10-only diagnostic text in `VehicleVisual3DController`.

## Safety invariants

- The four `RayCast3D` wheel nodes, their `wheel_node` paths, collision shapes,
  GEVP script, and all physics properties are unchanged.
- The scene preserves the right-side 180-degree wheel orientation contract
  checked by `VehicleVisualContractGuard`.
- The new controller is presentation-only and reads state through
  `VehicleStateReader`.

## Validation

- Godot headless editor import loaded the GDExtension and the new
  `VehicleVisual3DConfig` resource type.
- Headless load of `jordan_1995.tscn` exited 0. It could not instantiate its
  chassis/wheel visuals because this isolated worktree intentionally lacks the
  three ignored generated Jordan GLBs; the same baseline gap prevents direct
  runtime verification of the resolved model and wheel nodes here.
- No new parse error was reported for the resource, controller node, or changed
  scene hierarchy. The remaining errors are the established missing-GLB GEVP
  follow-on errors.
