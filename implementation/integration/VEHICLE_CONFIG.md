# vehicle_config.rs integration

Current state:
- `VehicleConfig` has `front_brake_bias`, `max_brake_torque`, `tire_pressure`, `tire_thermal`.
- `JsonBrakes` is strict (`deny_unknown_fields`) and currently only accepts brake torque/bias/ABS.

## Required imports

```rust
use crate::brake_thermals::BrakeThermalConfig;
```

## VehicleConfig

Append near the existing brake fields:

```rust
pub brake_thermal: BrakeThermalConfig,
```

Do not scatter duct fields across `VehicleConfig`; keep them inside `BrakeThermalConfig`.

## Canonical/default constructors

Add:

```rust
brake_thermal: BrakeThermalConfig::default(),
```

to every `VehicleConfig` constructor.

## JsonBrakes

Extend current strict struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct JsonBrakes {
    // existing fields...
    #[serde(default)]
    thermal: Option<JsonBrakeThermal>,
}
```

The most maintainable parser strategy is:

- create strict JSON mirror structs for `BrakeThermalConfig`,
  `BrakeAxleThermalConfig`, `BrakeDuctAxleConfig`;
- every leaf should be `Option<T>` so a partial JSON section overlays the Rust default;
- add `from_config` for serialization/round-trip tests.

Do NOT deserialize `BrakeThermalConfig` directly inside the outer vehicle JSON if that would bypass strict unknown-field validation.

## Validation

Reject:
- opening outside `[0,1]`;
- non-positive heat capacities;
- negative conductances / cooling gains;
- `optimal_min >= optimal_max`;
- `optimal_max > fade_start`;
- `fade_start >= critical`;
- invalid efficiencies outside `[0,1]`;
- non-positive inlet area;
- invalid drag/discharge coefficients.

## Tests

Add:
- partial brake thermal JSON uses defaults for omitted values;
- typo inside `brakes.thermal` is rejected;
- typo inside `front_duct` is rejected;
- `opening > 1.0` is rejected;
- JSON round-trip preserves front/rear openings.
