# simulation.rs integration

The supplied `simulation.rs` already has the desired tire thermal tick order.

## 1. VehicleState

Add:

```rust
pub brake_thermal: BrakeThermalSystem,
```

Initialize:

```rust
brake_thermal: BrakeThermalSystem::new(&config.brake_thermal),
```

## 2. Before/after powertrain braking

Before `st.powertrain.step_with_reaction(...)`, cache:

```rust
let brake_efficiency = st.brake_thermal.efficiency_scales();
```

Keep `step_with_reaction(...)` unchanged.

Immediately AFTER it returns, but BEFORE `process_wheel_torque(...)`:

```rust
for wheel in WheelIndex::ALL {
    let i = wheel as usize;
    st.powertrain.brake_torques[i] *= brake_efficiency[i];
}
```

This ensures:
- ABS still owns whether torque is pulsed to zero;
- thermal fade reduces the torque actually applied at the wheel;
- brake heat later uses the actual final torque.

Do not multiply `max_brake_torque` permanently.

## 3. Brake duct drag

After `st.aero.step(...)`, evaluate:

```rust
let vehicle_speed_ms = st.linear_velocity.length();
let brake_duct_drag_n = st.brake_thermal.total_duct_drag_force_n(
    &cfg.brake_thermal,
    cfg.air_density,
    vehicle_speed_ms,
);
```

When central drag is assembled:

```rust
let total_drag_n = st.aero.drag_force.abs() + brake_duct_drag_n;
```

Use `total_drag_n` in the existing opposing-velocity drag force.

Do NOT add duct drag to `coefficient_of_drag` every frame.
Do NOT add it to rolling resistance.

## 4. Brake thermal step

Immediately before the existing tire thermal block, create:

```rust
let mut brake_to_tire_heat = [BrakeToTireHeat::default(); 4];
```

For each wheel:

```rust
let tire_state = st.tire_thermal.wheels[i];

brake_to_tire_heat[i] = st.brake_thermal.step_after_braking(
    wheel,
    &cfg.brake_thermal,
    BrakeThermalInput {
        applied_brake_torque_nm: st.powertrain.brake_torques[i],
        wheel_angular_speed_rad_s: st.tires.wheels[i].spin,
        vehicle_speed_ms,
        air_density_kg_m3: cfg.air_density,
        ambient_temperature_c: cfg.tire_thermal.ambient_fallback_c,
        tire_carcass_temperature_c: tire_state.carcass_c,
        tire_gas_temperature_c: tire_state.gas_c,
    },
    dt,
);
```

## 5. Existing TireThermalInput

Add:

```rust
external_carcass_heat_w: brake_to_tire_heat[i].carcass_heat_w,
external_gas_heat_w: brake_to_tire_heat[i].gas_heat_w,
```

The brake and tire updates can share the same thermal stage because all fluxes are evaluated from pre-update node temperatures.

## 6. Reset

Where vehicle state is reset/reinitialized, ensure brake thermal state returns to configured initial temperature.
