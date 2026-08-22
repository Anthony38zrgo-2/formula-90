# Acceptance criteria — Brake Heat Transfer

## Energy source
- With applied brake torque = 0, brake friction heat generation = 0.
- With nonzero actual torque and wheel speed, brake power equals approximately `|T * omega|`.
- ABS pulse to zero torque also drops generated brake power to zero for that wheel.
- Fade-reduced torque produces proportionally less mechanical brake power.

## Thermal ordering
A hard braking event from cold must show:
1. disc rises fastest;
2. caliper/hub follow;
3. rim rises slower;
4. tire carcass/gas respond later.

No direct instantaneous tread spike from brake heat is allowed.

## Tire coupling
Repeated braking must eventually:
- raise rim temperature;
- raise tire carcass temperature;
- raise tire gas temperature;
- increase hot pressure relative to an otherwise identical no-braking run.

Front brake bias should naturally create more front brake heat when front torque is larger.

## Brake effectiveness
- cold disc: efficiency below 1.0;
- operating window: approximately 1.0;
- fade region: efficiency decreases;
- critical temperature: efficiency approaches configured minimum.
- current-tick efficiency comes from previous thermal state; no recursive same-tick brake solve.

## Duct cooling
At the same speed/temperature:
- larger opening -> larger effective area;
- larger opening -> larger mass flow;
- larger opening -> stronger brake/rim cooling.

Opening 0 still retains baseline cooling.

## Duct drag
At the same speed:
- larger opening -> larger duct drag.

At the same opening:
- doubling speed should produce ~4x duct drag before other aero interactions.

The cooling and drag calculations must use the same `effective_area`.

## Setup trade-off
A long run with smaller ducts should tend toward:
- higher disc temperature;
- higher rim temperature;
- more tire heat soak;
- higher tire gas temperature / pressure;
- lower duct drag;
- possible fade if too closed.

A long run with larger ducts should tend toward the inverse.

## Regression
Do not change:
- tricast ray count/placement;
- tire pressure gas law;
- existing tire thermal zone semantics;
- base engine/drivetrain torque;
- body/wing downforce model;
- ABS pulse logic except that thermal fade scales final applied brake torque.

## ABI
- physics ABI 8 -> 9.
- C mirror and Rust layout test updated together.
- core ABI inspected and bumped exactly once if its output struct changes.

## Determinism
At fixed dt and identical inputs, brake/tire thermal states must be deterministic.
No NaN/Inf is accepted.
