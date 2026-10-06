# physics_diagnostics — Physics and telemetry analysis

Scope: analyzers for aero, powertrain, suspension, tires, and telemetry, plus the runner executing them end to end.
Consumers: physics tuning iterations feeding `game/data/vehicles/`.
Rules: analyzers read telemetry captures; they never patch data files. Findings become data changes with tests, not silent edits.

`prepare_physical_world_package.py` produces source- and geometry-digested collision packages from track geometry. Preserve transforms, surface identity and digest verification; output candidates under `scratch/` until promotion is approved.

The coupled foundation and candidate review runners collect mass, dynamics, contact and timing evidence. Record vehicle profile, package provenance and explicit acceptance failures. Candidate results do not complete the 50-item backlog automatically. Keep full-grid performance failures visible alongside passing correctness gates. Suspension visual previews are evidence only and must be distinguished from full authored-scene validation.
