# Competition HUD

RetroHudDisplay is the competition HUD presentation used by the active race HUD. CompetitionHudAdapter supplies live vehicle and lap-timing values while the display remains presentation-only.

The display contains:

- a segmented diagonal revolutions-per-minute scale from 0 to 19,000
- a seven-segment gear and speed readout
- oil and water temperatures
- a segmented fuel gauge
- fuel remaining in kilograms
- average fuel consumption in kilograms per lap
- fuel delta in laps

The SVG resources provide independent vector parts. The seven-segment glyphs are composed from the reusable digital segment. Layout colors, redline, scale, and segment counts are configured in config/retro_hud.json.

The separate engine temperature panel is hidden by CompetitionHudAdapter because its temperatures and fuel figures are part of this display.
