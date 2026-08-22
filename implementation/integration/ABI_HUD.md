# ABI + HUD integration

## Vehicle physics ABI

Current supplied ABI is 8.

Appending brake thermal telemetry requires:
`F1_94_PHYSICS_ABI_VERSION = 9`

Append to `FfiTelemetryOutput`, never insert into the middle:

Per wheel:
- disc_c
- caliper_c
- hub_c
- rim_c
- brake_efficiency
- duct_mass_flow_kg_s
- duct_drag_n

Recommended field order:
- all FL/FR/RL/RR disc
- all caliper
- all hub
- all rim
- all efficiency
- all duct flow
- all duct drag

Update the Rust offset/size layout lock test and the C mirror.

## formula90-core

The uploaded RAR does not contain `formula90-core`, so inspect the current repository before changing its ABI.

If the previous tire-pressure integration has already moved core ABI 2 -> 3, append the brake block and bump:
`3 -> 4`.

Do not guess silently: verify the current constant/header pair first.

Propagate at least:
- brake disc temperatures [4]
- brake caliper temperatures [4]
- rim temperatures [4]
- brake efficiency [4]
- total brake duct drag

## Godot C++ vehicle

Expose:

```text
get_brake_state_snapshot()
```

Shape:

```gdscript
{
  "FL": {
    "disc_c": 465.0,
    "caliper_c": 180.0,
    "hub_c": 95.0,
    "rim_c": 72.0,
    "efficiency": 1.0,
    "duct_mass_flow_kg_s": 0.12,
    "duct_drag_n": 5.8
  },
  ...
}
```

## HUD

Keep the current tire panel above the tachometer.

Extend each wheel cell with one compact brake row:

```text
FL
P 109 kPa
I46 C49 O40
CAR34 GAS29
BRK D465 C180 R72
```

Where:
- `D` = disc
- `C` = caliper
- `R` = rim

Color the `BRK` row by brake operating range:
- cold: blue
- optimal: green
- above optimal but below fade: amber
- fade/critical: red

The HUD color thresholds MUST come from brake config values, not unrelated tire thresholds.

Do not create a second large brake panel.
