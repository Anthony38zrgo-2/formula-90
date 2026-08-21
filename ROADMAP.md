# Formula 90s — Development Roadmap

## Objective

Reach a stable **Simcade Foundation 1.0** before implementing race-oriented systems such as AI, race control, strategy, pit logic, championships, or opponent behavior.

The first development phase focuses entirely on building a complete simulation base:

```text
F1-94 Vehicle
    ↓
Vehicle Audio
    ↓
RealTimeTrack
    ↓
Dynamic Weather
    ↓
Environment Integration
    ↓
SIMCADE FOUNDATION 1.0
    ↓
Race Systems / AI
```

---

# M0 — F1-94 Vehicle Simulation Baseline

**Target:** August 22, 2026
**Status:** In Progress
**Estimated completion:** ~70%

## Goal

Finish the F1-94 as the reference vehicle for the simulation.

The vehicle must contain the complete mechanical, tire, aerodynamic, fuel, and powertrain simulation required by the rest of the project.

---

## VEH-100 — Mechanical System

Complete and validate interaction between:

- chassis
- suspension
- wheels
- tires
- steering
- brakes
- weight transfer
- contact patch
- raycast / tricast wheel model

The tire must behave as part of the suspension system rather than as a rigid grip generator.

---

## VEH-110 — Tire Model

Complete:

- tire pressure
- tire stiffness
- pressure-dependent vertical response
- pressure-dependent contact patch
- rolling resistance
- longitudinal grip
- lateral grip
- combined slip
- slip ratio
- slip angle
- tire load sensitivity

### Tire thermal state

Track independently:

- carcass temperature
- inner surface temperature
- center surface temperature
- outer surface temperature

Temperature must influence:

- grip
- pressure
- stiffness
- lateral response
- longitudinal response
- thermal degradation

---

## VEH-120 — Suspension Refinement

Complete:

- spring response
- damper response
- bump
- rebound
- suspension travel
- bump-stop interaction
- tire vertical compliance
- suspension + tire coupled response

The wheel/tire assembly must behave as another compliant component of the suspension.

---

# VEH-130 — Aerodynamics

The aerodynamic system already exists but requires final refinement.

## VEH-131 — Front Wing

Simulate:

- front downforce
- aerodynamic efficiency
- ride-height sensitivity
- pitch sensitivity

---

## VEH-132 — Rear Wing

Simulate:

- rear downforce
- drag
- aero balance
- setup-dependent behavior

---

## VEH-133 — Floor

Complete the floor interaction model.

The floor must react to:

- ride height
- pitch
- rake
- vehicle speed
- ground proximity

---

## VEH-134 — Diffuser

Implement dedicated ground-distance sampling.

The diffuser effectiveness must depend on actual distance between the floor and the track.

Possible states:

```text
Normal operating height
↓
Increasing ground effect
↓
Optimal operating window
↓
Flow instability
↓
Floor contact / stall
```

---

## VEH-135 — Aero Balance

Aerodynamic balance must change dynamically according to:

- pitch
- rake
- ride height
- speed
- suspension compression
- floor proximity
- wing configuration

Poor mechanical or aerodynamic setup must produce real handling consequences.

---

# VEH-140 — Fuel System

Implement fuel as part of the physical vehicle state.

## Requirements

- fuel quantity
- fuel consumption
- fuel mass
- fuel tank position
- dynamic vehicle mass
- center-of-mass variation
- fuel influence on handling
- fuel influence on acceleration
- fuel influence on braking

Fuel load must affect vehicle balance during a session.

---

# VEH-150 — Powertrain Refinement

Complete final powertrain behavior.

Include:

- torque curve
- power curve
- throttle response
- engine inertia
- drivetrain inertia
- gearbox ratios
- differential behavior
- engine braking
- RPM behavior
- torque delivery
- wheel torque

---

# VEH-160 — Telemetry

Expose enough telemetry to validate the entire car.

Minimum telemetry:

```text
wheel_load
wheel_travel
suspension_velocity
tire_pressure
tire_temperature
carcass_temperature

slip_ratio
slip_angle
contact_patch

front_downforce
rear_downforce
floor_downforce
diffuser_efficiency
aero_balance

ride_height_front
ride_height_rear
floor_distance

fuel_mass
vehicle_mass

engine_rpm
engine_torque
wheel_torque
```

---

# VEH-170 — Vehicle Integration Gate

Before closing the F1-94 milestone, perform complete integration testing.

The car must remain stable during:

- acceleration
- heavy braking
- trail braking
- high-speed cornering
- low-speed cornering
- curb impacts
- bumps
- elevation changes
- suspension compression
- aero compression
- tire temperature changes
- tire pressure changes
- fuel-load changes

---

# M1 — Vehicle Audio Engine 1.0

**Target:** 2–3 days after Vehicle Physics completion
**Estimated window:** August 23–25

## Goal

Replace the basic engine audio implementation with a complete vehicle sound system driven by simulation telemetry.

Audio must provide feedback about what the vehicle is physically doing.

---

# AUD-200 — Engine Audio

Implement:

- RPM-based engine layers
- load-dependent sound
- throttle-dependent sound
- acceleration state
- deceleration state
- intake
- exhaust
- engine braking
- RPM transitions
- rev limiter

---

# AUD-210 — Transmission Audio

Implement:

- gearbox whine
- gear engagement
- drivetrain load
- drivetrain unloading
- differential sound
- transmission inertia feedback

---

# AUD-220 — Mechanical Audio

Generate mechanical feedback from:

- suspension
- dampers
- chassis movement
- bottoming
- wheel impacts
- drivetrain
- gear changes

---

# AUD-230 — Tire Audio

Tire sound must react to:

- slip ratio
- slip angle
- wheel lock
- wheelspin
- surface type
- tire load
- tire speed

---

# AUD-240 — Surface Audio

Different materials must generate different rolling behavior.

Examples:

```text
asphalt
grass
gravel
sand
curb
wet asphalt
```

---

# AUD-250 — Collision Audio

Implement:

- light contact
- heavy contact
- floor strike
- suspension strike
- barrier contact
- body impact

Collision intensity must be derived from actual impact energy.

---

# M2 — RealTimeTrack

## Goal

Transform the circuit from static geometry into a continuously evolving physical surface.

RealTimeTrack becomes the authoritative state of the track.

The circuit must not have one global grip value.

Instead:

```text
Track
    ↓
Spatial Surface State
    ↓
Local Tire Interaction
```

---

# RTT-300 — Architecture Spike

Before implementing RealTimeTrack completely, define its architecture.

## Required decisions

- spatial representation
- grid / patch resolution
- memory budget
- update frequency
- vehicle query API
- weather query API
- shader integration
- persistence strategy
- telemetry
- debug visualization

Only after this task should final estimates for RealTimeTrack be established.

---

# RTT-310 — Surface State Grid

Divide the usable circuit into spatial surface regions.

Each region should be capable of storing:

```text
surface_material
surface_temperature

rubber_level
marbles_level
dirt_level

wetness
water_depth

grip_modifier

drainage
evaporation_rate

wind_exposure
sun_exposure
```

---

# RTT-320 — Rubber Evolution

Cars dynamically deposit rubber.

Rubber accumulation depends on:

- number of vehicles
- tire load
- tire slip
- racing line
- surface temperature
- surface material
- track state

Expected behavior:

```text
Cars circulate
↓
Rubber accumulates
↓
Racing line evolves
↓
Grip changes
```

---

# RTT-330 — Marbles

Generate rubber debris outside the main racing line.

Marbles should:

- accumulate gradually
- reduce grip
- interact with tires
- be displaced by cars
- react to rain

---

# RTT-340 — Dirt and Contamination

Cars leaving the track may bring dirt or debris back onto the racing surface.

Track contamination must be spatial.

Possible sources:

- grass
- gravel
- sand
- tire debris

---

# RTT-350 — Track Temperature

Track temperature must be calculated dynamically.

Inputs include:

- ambient temperature
- solar radiation
- cloud coverage
- wind
- surface material
- moisture
- water
- vehicle activity

Different parts of the circuit may have different temperatures.

---

# RTT-360 — Track ↔ Tire Interaction

This interaction must be bidirectional.

```text
Track → Tire

temperature
water
rubber
dirt
surface
grip
```

and:

```text
Tire → Track

heat
rubber
slip
dirt
water displacement
```

---

# RTT-370 — Dynamic Track Visuals

The visual state of the track must represent its physical state.

Visualize:

- rubbered racing line
- dirt
- marbles
- wet surfaces
- drying line
- standing water
- puddles

Visual effects must derive from simulation state rather than scripted animations.

---

# M3 — Dynamic Weather

## Goal

Implement a procedural weather system capable of generating spatial and emergent weather conditions.

Weather must not behave as a sequence of hardcoded global values.

Instead, weather scenarios define an intended evolution.

Example:

```text
Dry
↓
Increasing clouds
↓
Possible drizzle
↓
Localized rain
↓
Possible drying
```

The exact result is generated procedurally.

---

# WX-400 — Weather Scenario Controller

Provide high-level session weather scenarios.

Examples:

```text
Dry

Dry → Cloudy

Dry → Drizzle

Dry → Rain

Rain → Dry

Variable

Unstable
```

A scenario defines probability and direction of evolution, not exact values.

---

# WX-410 — Atmospheric State

Simulate environmental variables such as:

```text
ambient_temperature
humidity
pressure

wind_speed
wind_direction

cloud_density
precipitation_probability
```

---

# WX-420 — Dynamic Clouds

Clouds must exist spatially.

Clouds should:

- form
- move
- dissipate
- change density
- change direction with wind

Clouds should influence:

- track temperature
- sunlight
- precipitation probability

---

# WX-430 — Spatial Precipitation

Rain must exist in world space rather than as a global circuit variable.

Possible result:

```text
Sector 1 → Dry

Sector 2 → Light rain

Sector 3 → Heavy cloud / no rain
```

Rain may approach the circuit and never actually reach it.

---

# WX-440 — Wind

Wind must affect both environment and vehicle.

## Environment

Wind influences:

- clouds
- rain direction
- evaporation
- cooling
- track temperature

## Vehicle

Wind influences:

- relative airflow
- aerodynamic load
- stability
- tire cooling

---

# WX-450 — Water Deposition

Rain adds water to the RealTimeTrack surface state.

Water accumulation depends on:

- rainfall intensity
- track geometry
- slope
- drainage
- evaporation
- traffic

---

# WX-460 — Drainage

Water must move or disappear according to:

- gravity
- track slope
- drainage properties
- surface material
- evaporation

---

# WX-470 — Puddles

Standing water should emerge naturally from circuit geometry.

Expected behavior:

```text
Rain
↓
Water accumulation
↓
Low point in geometry
↓
Poor drainage
↓
Puddle
```

Puddles should not require manually scripted placement wherever possible.

---

# WX-480 — Rubber Wash-Off

Rain must progressively remove accumulated rubber.

Example:

```text
Rubbered racing line
↓
Localized rain
↓
Rubber degradation
↓
Lower local grip
```

This can make one sector significantly more dangerous than another.

---

# WX-490 — Drying Line

Cars should actively influence drying.

Expected behavior:

```text
Wet track
↓
Cars circulate
↓
Water displacement
+
tire heat
+
air movement
↓
Racing line dries first
```

---

# M4 — Environment Integration

## Goal

Validate the complete environmental simulation as one interconnected system.

The following chain must work:

```text
Atmosphere
    ↓
Weather
    ↓
RealTimeTrack
    ↓
Tire
    ↓
Suspension
    ↓
Vehicle
```

And interactions must also travel in the opposite direction where appropriate.

---

# INT-500 — Weather → Track

Weather must modify:

- temperature
- water
- rubber
- contamination
- drying
- grip

---

# INT-510 — Track → Tire

The tire must react to local:

- temperature
- water depth
- rubber level
- dirt
- grip
- surface material

---

# INT-520 — Tire → Track

Tires must dynamically influence:

- rubber accumulation
- dirt displacement
- heat
- water displacement
- drying

---

# INT-530 — Weather → Tire

Environmental conditions must affect tire thermal behavior.

Examples:

```text
cold air → greater cooling

rain → greater surface cooling

wind → increased heat transfer

hot asphalt → increased tire heating
```

---

# INT-540 — Weather → Aerodynamics

Wind must modify relative airflow around the vehicle.

This may change:

- effective airspeed
- downforce
- drag
- aero balance
- high-speed stability

---

# INT-550 — Integrated Validation

Test emergent scenarios.

## Scenario A — Localized rain

```text
Cloud reaches Sector 2
↓
Sector 2 receives rain
↓
Surface temperature drops
↓
Rubber is progressively removed
↓
Water accumulates
↓
Tires cool
↓
Grip decreases
↓
Sector becomes more dangerous
```

---

## Scenario B — Drying race line

```text
Rain stops
↓
Wind + evaporation begin drying
↓
Cars circulate
↓
Racing line dries faster
↓
Offline areas remain wet
↓
Grip becomes spatially asymmetric
```

---

## Scenario C — Puddle formation

```text
Heavy rain
↓
Track depression accumulates water
↓
Drainage insufficient
↓
Standing water forms
↓
Vehicle crosses puddle
↓
Grip and tire behavior change
```

---

# M5 — SIMCADE FOUNDATION 1.0

This is the first major Formula 90s development milestone.

## Required systems

### Vehicle

- mechanical simulation
- suspension
- tire model
- pressure
- temperatures
- brakes
- weight transfer
- aerodynamics
- floor
- diffuser
- fuel
- powertrain

### Audio

- engine
- drivetrain
- tires
- suspension
- collisions
- surfaces

### RealTimeTrack

- rubber
- marbles
- dirt
- track temperature
- wetness
- water
- drainage
- puddles
- dynamic grip

### Weather

- atmospheric state
- procedural weather progression
- dynamic clouds
- spatial precipitation
- wind
- rain
- drying

### Integration

- Weather ↔ Track
- Track ↔ Tire
- Tire ↔ Track
- Weather ↔ Tire
- Weather ↔ Aerodynamics

---

# Definition of Done — Simcade Foundation 1.0

The milestone is complete when Formula 90s can simulate a complete driving session in which the vehicle and environment evolve continuously without depending on scripted grip changes.

A valid session may produce:

```text
Dry track
↓
Rubber accumulation
↓
Increasing track grip
↓
Cloud coverage
↓
Localized temperature decrease
↓
Rain in part of the circuit
↓
Rubber wash-off
↓
Wet surface
↓
Puddle formation
↓
Tire cooling
↓
Grip reduction
↓
Rain stops
↓
Track gradually dries
↓
New racing line develops
```

All of these states must emerge from the simulation.

---

# Phase 2 — Race Simulation

Race-related features start only after **Simcade Foundation 1.0** is stable.

---

# RACE-600 — Race Systems

Future systems:

- sessions
- practice
- qualifying
- race
- grid
- race start
- lap counting
- timing
- sectors
- penalties
- flags
- race control

---

# AI-700 — Driver AI

First major system after the simulation foundation.

AI development should eventually include:

- racing line
- braking behavior
- overtaking
- defending
- traffic awareness
- tire management
- fuel management
- weather adaptation
- wet-line adaptation
- risk
- mistakes
- driver traits

The AI should consume the same physical information available to the player rather than relying on artificial grip or simplified physics.

---

# Future Systems

After race AI:

```text
Pit System
Strategy
Damage
Reliability
Race Control
Championship
Drivers
Teams
Car Variants
Circuits
Career / Season Structure
```

---

# High-Level Roadmap

```text
AUGUST 2026

19 ───────────────────── 22
F1-94 VEHICLE SIMULATION

                         23 ─── 25
                         VEHICLE AUDIO

                                  26 ── 27
                                  REALTIMETRACK
                                  ARCHITECTURE

                                          ↓

                         REALTIMETRACK CORE
                         Rubber
                         Dirt
                         Marbles
                         Temperature
                         Dynamic Grip

                                          ↓

                         DYNAMIC WEATHER
                         Clouds
                         Wind
                         Spatial Rain
                         Water
                         Drainage
                         Puddles

                                          ↓

                         ENVIRONMENT INTEGRATION

                                          ↓

                         SIMCADE FOUNDATION 1.0

                                          ↓

                         RACE SYSTEMS

                                          ↓

                         DRIVER AI
```

---

# Project Rule

Until `SIMCADE FOUNDATION 1.0` is completed:

> Priority is simulation depth, system interaction and telemetry. Race-oriented systems must not become the primary development focus before the underlying vehicle and environmental simulation are stable.
