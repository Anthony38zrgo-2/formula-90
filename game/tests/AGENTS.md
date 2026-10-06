# tests — Godot-side test suites

Scope: GDScript smoke, visual-capture, audit, and verification suites run in Godot, plus the GdUnit framework suites.
Consumers: the `scripts/test_gdunit.*` gates and human visual review.
Rules: tests observe the running game; shared engine assertions live in `game/tests/gdunit/`. Capture tests produce evidence, not golden screenshots committed blindly.
Subfolders: each child folder documents itself in its own AGENTS.md.

## Physical-world integration gates

- `test_physical_vehicle_world_interface.gd` exercises the native world interface.
- `test_physical_world_controller.gd` checks scheduling, input ordering, pause, services and removal with mock vehicles.
- `test_physical_world_native_presentation.gd` uses two real vehicle/core pairs, a source-verified package and the actual wheel/link renderers. Check frozen proxies, authoritative poses, services and zero second linkage solves.
- Run the canonical F1 2030/Fuji smoke separately after a clean production rebuild. These gates do not replace sustained driving, rendered whole-car review, impact coverage or full-grid performance acceptance.

Inspect error output as well as exit status and PASS markers. The standalone native presentation harness currently triggers telemetry-autoload errors for `front_left_wheel` and `get_telemetry_snapshot` on its test vehicles; record this limitation until the harness or telemetry contract is corrected. Do not claim an error-free runtime from its PASS marker alone.
