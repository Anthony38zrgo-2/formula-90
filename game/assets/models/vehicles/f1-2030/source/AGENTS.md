# source — 2030 authoring sources

Scope: Blender sources and texture packs behind the 2030 meshes and liveries.
Consumers: `tools/blender/` remodel, livery, and validation scripts.
Rules: sources regenerate the runtime meshes; review renders are evidence, not releases. The driver-fitted seat is regenerated with tools/blender/fit_f1_2030_driver_seat.py; driver_seat_fit_report.json records the protected geometry and animated-driver clearance checks.

The cockpit clearance is regenerated with tools/blender/fit_f1_2030_cockpit_clearance.py. It captures the current driver in an isolated Godot project and records source, animation and geometry hashes in cockpit_clearance_report.json. Repeating the fitting requires --source-file pointing to the original source recorded by that report, so the adjustments are applied once. The operation fits GEO_CHASSIS_BODY, GEO_CHASSIS_INTERIOR and GEO_CHASSIS_FLOOR, translates the seat and complete steering wheel assembly rearward by 100 millimeters, and extends GEO_CHASSIS_STEERCOLUM while preserving its forward anchor. The matching seated_position belongs to the vehicle scene; the driver mesh and skeleton proportions remain unchanged. Promotion requires no intersections across the animated driver samples or between the seat and surrounding cockpit meshes. Other meshes remain protected.
