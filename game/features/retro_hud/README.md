# Retro HUD — Formula broadcast 2004–2008

`RetroHudDisplay` remains presentation-only. The race HUD adapter provides:

- speed in km/h
- engine RPM
- gear
- analog throttle input (0–1)
- analog brake input (0–1)

The display is now procedural and intentionally does **not** depend on texture frames from the Assetto Corsa reference mod. It recreates the useful visual grammar of the 2004–2008 broadcast HUD:

- circular 6k–20k RPM scale
- white live RPM needle
- red peak-hold RPM needle
- white gear plate
- green analog throttle bar
- red analog brake bar
- 20-segment speed arc with green/yellow/orange/red regions
- numeric speed and 200/260/320/340 scale labels

Visual/behaviour tuning lives in `config/retro_hud.json`.

The existing `ArcadeRaceHud` is still the runtime adapter and `RetroHudDisplay` still does not resolve the vehicle itself.
