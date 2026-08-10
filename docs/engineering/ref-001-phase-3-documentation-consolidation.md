# REF-001 Phase 3 — documentation consolidation

## Verified current contract

- GEVP GDScript owns physical vehicle simulation.
- C++ GDExtension owns bootstrap, camera, visual presentation, audio, reset,
  and menu integration as vehicle-state consumers.
- `WorldHudCompositor` separates the fixed-resolution world from the root
  `HudLayer`.
- The active gameplay HUD and minimap are GDScript controls.
- `DirectionalVehicleSprite` remains used by static validation; it is not the
  player renderer.
- `DebugHudController` and `StaticMinimapController` are still registered, but
  no current gameplay scene references them. This is a later reference-audit
  decision, not a removal performed by this phase.

## Changes made

- Replaced `docs/architecture.md` with the current runtime ownership model and
  source-of-truth hierarchy.
- Rewrote the README architecture and test-status sections to remove the
  obsolete all-C++ runtime claim.
- Clarified the architecture overview and telemetry status.
- Preserved historical migration plans, audits, and V10 camera notes while
  explicitly labelling them as non-authoritative for current implementation.

## Validation

- Searched all affected documentation for the former active
  `ArcadeCarController` claims.
- The only remaining matches are within files now marked historical/proposal.
- `git diff --check` passed.
- This phase changes documentation only; no scene, physics, native code, or
  asset behavior was modified.
