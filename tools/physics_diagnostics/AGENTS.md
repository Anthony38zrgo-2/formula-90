# physics_diagnostics — Physics and telemetry analysis

Scope: analyzers for aero, powertrain, suspension, tires, and telemetry, plus the runner executing them end to end.
Consumers: physics tuning iterations feeding `game/data/vehicles/`.
Rules: analyzers read telemetry captures; they never patch data files. Findings become data changes with tests, not silent edits.
