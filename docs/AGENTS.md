# docs — Architecture and delivery evidence

Scope: repository architecture, implementation backlogs, validation boundaries and agent handoffs.
Consumers: implementation agents and human reviewers.
Rules: documents describe the verified source state and distinguish implementation, automated validation and human acceptance. Preserve unresolved gates and source provenance when updating delivery status.

## Coupled vehicle work

`coupled_vehicle_backlog_execution_status.json` tracks the 50-item backlog. Update individual evidence and acceptance fields against actual results; passing a candidate suite does not imply all items or human approval are complete.

`coupled_vehicle_godot_integration.md` describes the optional physical-world ownership and presentation contracts. Keep activation, service routing, source parity and validation limits aligned with code. The candidate remains opt-in; sustained driving, whole-car visual review and full-grid performance require separate acceptance.

Rebuild evidence belongs under `scratch/coupled_vehicle_godot_runtime_rebuild/`. Its production build and canonical smoke passed with uncommitted integration source. The native two-car contract passed but recorded telemetry-autoload errors. Distinguish this later evidence from the earlier isolated integration harness; never infer an error-free or approved production runtime from either result.
