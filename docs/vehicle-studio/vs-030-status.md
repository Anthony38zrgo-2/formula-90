# VS-030 — VehicleBuildIR compiler

Status: READY_FOR_HUMAN_GATE

## Outcome

A pure compiler now resolves VehicleDocument parameters into an ordered,
canonical VehicleBuildIR. Each operation declares its inputs, dependencies and
postconditions; Blender is not allowed to reinterpret semantic intent.

The Williams revision declares seven initial global parameters:

- wheelbase;
- front and rear track;
- front and rear tire radius;
- front and rear tire width.

Every parameter exposes absolute metres, percentage bounds and explicit
operations. The Vue inspector supports numeric entry and percentage sliders.
It only compiles a plan; it does not launch Blender, write artifacts or publish.

## Williams proof

A wheelbase target of 105 percent compiled to 3.06668357625 m and produced 13
ordered operations. BuildIR SHA-256:
3CCC371829D4BD2C09EFB1775C7794179E4553F72F15DE67098B1B4A8F35345A.

The plan includes source assertion, tire/track operations, axle translation,
piecewise body deformation, validation and staging-only export.

## Launcher correction

The PowerShell script now delegates process ownership to a Node runner that
controls Vite and Python directly. A Ctrl+C integration test left no listeners
on either test port, eliminating the observed orphan API process.

## Verification

- 58 Python tests pass.
- 6 Vue tests pass.
- Production TypeScript/Vite build passes.
- Unknown, non-finite and out-of-bounds parameters are rejected.
- Dependency order, unique operations and postconditions are validated.
- Equivalent inputs produce identical canonical bytes.
