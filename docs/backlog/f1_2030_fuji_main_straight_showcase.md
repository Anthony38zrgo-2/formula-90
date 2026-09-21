# Backlog item: F1 2030 Fuji main-straight showcase

Status: implementation steps 1-3 in progress; clip review and human gate belong to the user.

## Goal

Create a roughly 30-second, one-shot showcase of the F1 2030 car and its existing runtime audio on Fuji's main straight. Show idle, launch, a controlled slalom, acceleration with real gear changes, and braking with downshifts.

## Scope

- Use `game/tracks/fuji76_77/metadata/racing_line.json` for the demo route and the existing Fuji runtime scene for track visuals, collision, lighting, and world environment.
- Start near the upstream end of the main straight at racing-line waypoint 650; follow the continuous metadata sequence through waypoint 818, wrap to waypoint 0, and finish the guided path at waypoint 75. The metadata arc length is about 1,296.8 m.
- Add four non-colliding visual slalom gates over the first 260 m of the run. Keep the path offset within the metadata's lateral boundaries.
- Drive through the existing vehicle input/physics interface; never animate or teleport the car during the run. Enable the configured automatic transmission for physical upshifts/downshifts and keep the existing `F90Core` audio path.
- Use a moving, low 3/4-front camera and hide the debug HUD for the showcase.
- Launch through the canonical `scripts/run_f1_94.ps1` entry point so its asset and BUILD checks remain in force.

## Acceptance and handoff

- A dedicated scene/replay runs once for approximately 30 seconds on Fuji without changing the canonical track, vehicle model, physics profile, or audio implementation.
- A static route audit confirms the chosen slalom section fits within the racing-line widths.
- Runtime capture, perceptual review, final render approval, and sprint retrospective remain pending for the user's review stage.
