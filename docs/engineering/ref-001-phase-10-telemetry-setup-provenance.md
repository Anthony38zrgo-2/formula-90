# PHY-001 — telemetry setup provenance

## Change

`TelemetryManager` now creates an immutable `<session>_setup.json` when it
opens each CSV capture. The snapshot reads actual runtime vehicle values and
contains:

- scene and vehicle provenance, plus optional engine/torque-resource paths;
- chassis, steering, brakes, differential, suspension, tires/surfaces, engine,
  transmission, and aerodynamics families;
- the active driving-aid state when a matching controller is present.

The manager creates a stable JSON signature without timestamp data. When a
captured runtime setup value changes, it flushes/closes the old CSV and starts
a new CSV + setup pair. Existing files are never overwritten or mutated.

## Validation

`tests/smoke_test_telemetry_setup_pairing.gd` instantiates the Jordan handling
scene, waits for automatic vehicle resolution, and verifies:

1. a first JSON is paired to an existing CSV;
2. the JSON has schema version 1 and required provenance/setup/assist fields;
3. changing `brake_force_multiplier` produces a distinct second pair.

The validated local runtime snapshots recorded Jordan Phase B, 505 kg, six
ratios, and a brake multiplier transition from `1.0` to `1.125`. Generated
CSV and JSON evidence stays under ignored `game/telemetry/`; it is not
committed as baseline data.

## Running the gate

After materializing the local Jordan runtime assets, run:

```powershell
& D:\Formula90s\.tools\godot\Godot_v4.7.1-stable_win64_console.exe --headless --path game --script tests\smoke_test_telemetry_setup_pairing.gd
```

Use the resulting paired capture before any physics-tuning change. The missing
native `physics_math.hpp` header does not block this telemetry gate.
