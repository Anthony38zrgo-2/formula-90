# Formula90s handling-development circuit

Scene:

`res://scenes/tracks/test_field/formula90s_test_track.tscn`

Builder:

`res://scenes/tracks/test_field/formula90s_test_track.gd`

The Jordan handling test uses this circuit instead of the GEVP vendor demo track. The vendor scene at `res://addons/gevp/scenes/track.tscn` remains untouched and can still be used as a reference/baseline asset.

## Purpose

This circuit is a physics-development sandbox, not an FIA-homologated venue. Its geometry follows real-circuit design principles where useful for handling tests:

- asphalt is bounded by continuous white edge lines;
- grass/run-off can be reached smoothly when the car crosses the track edge;
- curbs are local corner features rather than continuous strips around the lap;
- barriers are selective and leave deliberate open run-off zones;
- longitudinal ramps are excluded from the normal lap;
- banking is produced by track crossfall/tilt, not by raising the centreline into an artificial jump.

Reference principles: FIA International Sporting Code Appendix O 2026, especially Articles 7.3, 7.6 and 7.8.

## Track width

The road is 12 m wide (6 m from the centreline to each edge). This follows the FIA Appendix O general recommendation for a new permanent circuit and gives an F1-sized car enough room for multiple lines during handling work.

The circuit keeps a constant width so there are no abrupt width transitions affecting steering tests.

## Track edge and grass

A continuous white line marks both road edges.

Beyond the white line is grass. There is no artificial wall or curb around the whole circuit. This lets the handling test cover:

1. two wheels on asphalt / two wheels on grass;
2. progressive transition from asphalt grip to grass grip;
3. recovery after a minor track-limit excursion;
4. braking with one side of the car on a lower-grip surface;
5. rejoining without an artificial barrier interaction.

The grass plane and asphalt centreline are effectively level; the asphalt is raised only a few millimetres to avoid visual z-fighting.

## Curbs

Curbs are 0.55 m wide and approximately 25 mm above the road surface. They use only the `Curb` surface group so GEVP reads the configured curb tire coefficients rather than treating them as normal Road.

Curbs are used at selected:

- inside apexes;
- corner exits where a driver would naturally open the steering;
- direction-change complexes where curb interaction is useful for suspension testing.

Curbs are deliberately absent from:

- normal straights;
- long sections with no meaningful apex;
- every outside edge merely for decoration;
- open grass-testing zones.

Current curb test zones:

- first major right-hander: inside + exit;
- following left-hander: inside + exit;
- long right sweeper: inside + exit;
- middle right/left complex;
- final banked sector: inside + exit.

This gives enough curb events to test suspension and tire transitions without making every metre of the lap a curb strike.

## Banking

All centreline points use Y=0, eliminating the old +3 m ramp in the final sector.

Banking is applied only through `Curve3D` tilt and uses moderate values:

- first large corner: up to about 7 degrees;
- long sweeper: up to about 6 degrees in the opposite direction;
- final sector: up to about 9 degrees.

The purpose is to test combined lateral load, suspension compression and grip on a banked surface while keeping the normal lap free of jumps.

## Barriers and open run-off

Guardrails are not continuous.

They remain only in selected areas where a defined high-speed boundary is useful:

- both sides of the main straight;
- outside of the fast sweeper;
- outside of the final banked sector.

Large parts of the circuit intentionally have no nearby guardrail. Those sections are the primary grass/run-off handling test areas.

This follows the real-circuit principle that protection is selected according to expected trajectory, speed and impact conditions; run-off is particularly important outside corners. Distances in this sandbox are compressed for development convenience and are not intended to reproduce FIA homologation dimensions.

## Surface groups

- asphalt: `Road`
- curb: `Curb`
- surrounding ground: `Grass`
- guardrail: `Wall`

Do not assign both `Road` and `Curb` to the same curb node. The current GEVP wheel implementation reads the first recognised surface group, so a dedicated `Curb` group removes ambiguity.

## Phase B / Phase C validation use

Before Phase C steering tuning, use this track to validate:

1. flat straight-line acceleration and braking;
2. small grass excursion with two wheels outside the white line;
3. full-car grass excursion and controlled rejoin;
4. curb crossing at low and medium speed;
5. banked corner compression and grip;
6. recovery after clipping an exit curb;
7. high-speed boundary interaction only where guardrails exist.

If a handling anomaly appears only on a curb or barrier, do not immediately retune road tire grip. First distinguish Road, Curb, Grass and direct chassis collision behavior.
