# Brake duct cooling / drag correlation

Do not build two unrelated curves.

`brake_thermals.rs::evaluate_duct_flow()` is the single source of truth.

## Effective area

```text
A_eff = A_max * opening ^ area_response_exponent
```

## Dynamic pressure

```text
q = 0.5 * rho * v^2
```

## Ram mass flow

```text
delta_p = q * pressure_recovery
m_dot = discharge_coefficient * A_eff * sqrt(2 * rho * delta_p)
```

## Cooling

The brake node air conductance uses normalized mass flow:

```text
h = h_base + h_flow_gain * flow_ratio ^ cooling_flow_exponent
```

There is a `minimum_cooling_flow_ratio` so a closed duct is not a thermos.

## Drag

Per wheel:

```text
F_duct = q * duct_drag_coefficient * A_eff
```

Total duct drag is the four-wheel sum.

This gives the required physical correlation:
- opening ↑ -> effective area ↑;
- mass flow ↑ -> cooling ↑;
- CdA ↑ -> drag ↑;
- speed ↑ -> both ram effect and drag increase;
- drag grows approximately with v².

## Ownership

Keep the base body/wing aero in `aero.rs`.
The duct helper can stay in `brake_thermals.rs` because cooling and drag must consume the same opening geometry.

`simulation.rs` simply adds the resulting scalar duct drag to the existing central drag force.

Future work may distribute duct drag spatially per wheel, but that is NOT required for this milestone.
