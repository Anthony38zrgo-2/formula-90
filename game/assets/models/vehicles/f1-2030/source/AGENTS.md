# source — 2030 authoring sources

Scope: Blender sources and texture packs behind the 2030 meshes and liveries.
Consumers: `tools/blender/` remodel, livery, and validation scripts.
Rules: sources regenerate the runtime meshes; review renders are evidence, not releases. The driver-fitted seat is regenerated with tools/blender/fit_f1_2030_driver_seat.py; driver_seat_fit_report.json records the protected geometry and animated-driver clearance checks.
