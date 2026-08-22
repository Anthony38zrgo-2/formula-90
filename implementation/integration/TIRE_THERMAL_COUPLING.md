# tire_thermals.rs — external brake heat hook

The current tire thermal solver already has the correct thermal nodes. Do not add a separate "brake-heated tire" state.

Extend `TireThermalInput`:

```rust
pub external_carcass_heat_w: f64,
pub external_gas_heat_w: f64,
```

Then change the existing carcass balance:

```rust
let carcass_net_w =
    tread_to_carcass_w
    + flex_power_w
    + input.external_carcass_heat_w
    - carcass_to_gas_w
    - carcass_to_air_w;
```

And gas balance:

```rust
let gas_net_w =
    thermal.carcass_to_gas_w_k * (st.carcass_c - st.gas_c)
    + input.external_gas_heat_w;
```

Rules:
- positive watts mean heat ENTERING tire;
- do not write directly to tread temperature from brake;
- do not convert brake heat into a temperature delta in `simulation.rs`;
- `brake_thermals.rs` must subtract the same rim->tire watts from the rim energy balance, so energy is not created twice.

## Important correction while touching this file

The current `zone_contact_weights` recommendation is `[1,2,1]`.
That 1:2:1 is appropriate for geometric tricast support but should not automatically mean 50% of thermal load always belongs to the center.

For Brake Heat Transfer this is not mandatory to rewrite, but the agent should keep brake heat out of the zone weighting entirely:
- brake heat -> carcass/gas;
- tire/road/slip heat -> I/C/O.
