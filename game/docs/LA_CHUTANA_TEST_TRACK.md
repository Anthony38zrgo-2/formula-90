# La Chutana handling test track

Purpose: use a real Peruvian circuit shape as the Formula90s handling-development reference before Phase C steering/countersteer work.

## Active scene

`res://scenes/tracks/test_field/jordan_handling_test.tscn`

Track:

`res://scenes/tracks/test_field/la_chutana_track.tscn`

Builder:

`res://scenes/tracks/test_field/la_chutana_track.gd`

## Reference data

The current reconstruction uses the user-provided top-down layout as the geometric reference and public circuit data as scale anchors.

Targets used:

- lap length: approximately **2.420 km**;
- main straight: approximately **800 m**;
- reference turn count: **7**;
- current direction: **clockwise**.

Some public databases disagree on secondary measurements, so this scene should be treated as a gameplay/physics reconstruction rather than survey-grade CAD.

Published elevation information is deliberately not reproduced yet. The current objective is to validate the car without artificial ramps; only subtle local banking remains.

## Layout reconstruction

The supplied top-down image was manually traced into a centerline.

The trace is calibrated independently along its two image axes so that:

- the full control polyline is approximately 2.420 km;
- the long top straight is approximately 800 m;
- the proportions of the supplied layout remain close to the reference image.

The runtime prints the baked spline length and estimated main-straight length so later refinements can be measured instead of guessed.

## Surface

- 12 m development-track width.
- White edge lines define asphalt limits.
- Grass/run-off begins immediately outside the circuit where no curb exists.
- No guardrails are generated in this first La Chutana iteration. This is intentional so Road -> Grass -> Road behavior can be tested safely.

The 12 m width is a Formula90s development choice, not a claim about La Chutana's surveyed width.

## Curbs

The old generic test curbs were too abrupt for a low Formula chassis.

La Chutana uses a new low crowned curb profile:

- width: ~0.58 m;
- maximum positive rise: ~22 mm;
- road-side approach: +5 mm;
- shallow crown: +22 mm;
- outer approach returns to ~+2 mm before grass;
- no rectangular 50+ mm wall at either drivable edge.

This is intentionally conservative. It is inspired by conventional positive racing-curb geometry but is **not** claimed to reproduce a specific FIA-homologated curb drawing.

Curbs exist only at selected apex and exit zones; they do not run around the whole lap.

## Start / finish

A procedural black-and-white checker texture marks start/finish.

The Jordan spawns roughly 18 m before the line on the main straight with default vehicle orientation, so forward motion crosses the line and continues toward Turn 1.

## Validation before Phase C

1. Start approximately 18 m before meta and cross it naturally with forward throttle.
2. Complete a full lap without hidden ramps or seams.
3. Compare the visual shape against the supplied La Chutana top view.
4. Touch each curb with two wheels at low, medium and higher speed.
5. A normal curb touch may unsettle the car, but must not catapult it.
6. Run two wheels onto grass and return to asphalt.
7. Run fully onto grass and recover.
8. Confirm Road/Curb/Grass surface groups continue producing distinct GEVP behavior.
9. Check the console-reported baked lap and straight lengths before making further geometry changes.
