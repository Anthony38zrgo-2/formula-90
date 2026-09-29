# Competition HUD

RetroHudDisplay is the competition HUD presentation used by the active race HUD. CompetitionHudAdapter supplies live vehicle and lap-timing values while the display remains presentation-only.

The display contains:

- a segmented diagonal revolutions-per-minute scale from 0 to 19,000, with the red band beginning and lighting up at 17,800
- two shift lights at the upper-left of the tachometer: white from 17,400 and red from 17,800 revolutions per minute
- large gear and speed readouts in Barlow Condensed Medium
- oil and water temperatures
- a segmented fuel gauge with partial-segment fill based on remaining fuel divided by tank capacity
- fuel remaining in kilograms
- average fuel consumption in kilograms per lap
- fuel delta in laps

The gear turns red with the urgent shift light from 17,800 revolutions per minute. Oil and water values follow the engine's temperature ranges and the same cold, optimal, warm, and hot color palette as the tire readings. The SVG resources provide independent vector parts for the revolutions-per-minute scale, fuel gauge, and dividers. All HUD text uses Barlow Condensed Medium. Layout colors, redline, shift thresholds, scale, and segment counts are configured in config/retro_hud.json.

The separate engine temperature panel is hidden by CompetitionHudAdapter because its temperatures and fuel figures are part of this display.
