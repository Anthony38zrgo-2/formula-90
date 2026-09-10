# Formula90s HUD 2004–2008 conversion

Modified files:

- `game/scripts/hud/arcade_race_hud.gd`
  - forwards `throttle_input` and `brake_input` to the HUD.
- `game/scenes/ui/debug_hud.tscn`
  - resizes/repositions the primary HUD for the tall broadcast cluster.
- `game/features/retro_hud/scenes/retro_hud_display.tscn`
  - replaces the old LCD composition with one procedural drawing surface.
- `game/features/retro_hud/scripts/retro_hud_display.gd`
  - implements RPM dial, live needle, red peak-hold needle, gear plate, throttle/brake bars and segmented speed arc.
- `game/features/retro_hud/scripts/retro_hud_state.gd`
  - adds throttle and brake state.
- `game/features/retro_hud/scripts/retro_hud_config.gd`
  - adds 2004–2008 HUD tuning parameters.
- `game/features/retro_hud/config/retro_hud.json`
  - new visual/behaviour preset.

No graphics from the Assetto Corsa mod were copied into this package. The result is fully procedural and therefore avoids the original mod's ~350 full-frame PNG animation approach.
