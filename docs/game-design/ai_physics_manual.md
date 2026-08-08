# AI Physics Manual — Anti-Hallucination Guide for Agents (GEVP)

**This manual exists to override the semantic biases of Large Language Models (LLMs).**
When diagnosing physics issues in this project, **DO NOT** rely on your linguistic intuition. Strictly use the mechanical laws documented here.

## 1. Anti-Hallucination Parameter Dictionary (False Intuition vs Reality)

### `bump_stop_multiplier`
- **LLM's False Intuition:** "Multiplier sounds violent. If the car bounces too much, I should lower this multiplier to soften it."
- **Physical Reality:** The *bump stop* is the hardened rubber at the end of the spring's travel. It acts as an exponential resistance wall. **Increasing** this multiplier stops the wheel *before* the suspension bottoms out on the metal. **Decreasing** this multiplier to 1.0 removes the resistance, allowing the suspension to collapse instantly and the metal chassis (`CollisionShape3D`) to strike the asphalt, causing infinite G-force spikes and catastrophic rollovers.
- **Rule:** If the chassis hits the ground (bottom-out), you must INCREASE the bump stop (e.g., from 3.0 to 5.0).

### `resting_ratio`
- **LLM's False Intuition:** "If the car rides low, I should increase the ratio to 100% (1.0)."
- **Physical Reality:** Defines what percentage of the `spring_length` is consumed under the static weight of the car. A `resting_ratio` of 0.25 means the spring is so stiff that the car's weight only compresses it by 25%. Low values = Very stiff springs (High Spring Rate). High values (>0.6) = Very soft springs.
- **Rule:** Never exceed 0.7, or the car will lack travel to absorb bumps.

### Ground Clearance (`CollisionShape3D` vs Wheels)
- **Physical Reality:** Ground clearance is defined by the distance between the bottom edge of the main `CollisionShape3D` (the chassis) and the effective position of the wheel raycasts.
- **Rule:** If the suspension has a `spring_length` of 0.15m, the chassis must have *at least* 0.15m of ground clearance. Otherwise, when the wheel compresses, the chassis will scrape the ground. Adjust the `Transform3D` (Y axis) of the `CollisionShape3D` to correct this.

### Damping Ratio vs Absolute Damping (The classic LLM error)
- **LLM's False Intuition:** "If the rear axle is heavier than the front, I must multiply the front `damping_ratio` by 5 so it brakes with the same force."
- **Physical Reality:** In GEVP, the `damping_ratio` parameter is a **dimensionless coefficient ($\zeta$)**, not an absolute force in Newtons. The physics engine automatically calculates the absolute force by multiplying $\zeta$ by the mass and stiffness.
- **Rule:** NEVER attempt to compensate for mass differences between axles by skyrocketing the `damping_ratio` above `1.0`. If you want both axles to have the same oscillation braking capacity, use exactly the SAME value for both (e.g., `0.78`), and the physics engine will magically adjust the force based on the weight of each wheel. Values > 1.0 will freeze the suspension.

## 2. Symptoms and Diagnostics Table (Symptom -> Cause -> Solution)

| Observed Symptom (or reported by user) | Real Mechanical Cause | Solution to Implement |
| :--- | :--- | :--- |
| **The car constantly pitches back and forth (Pitch oscillation)** | **Unbalanced natural frequencies.** One axle (usually the rear) vibrates faster than the front. The phase difference amplifies the movement. | Calculate frequencies and adjust the `resting_ratio` of one axle so both oscillate at nearly the same Hz (e.g., both at ~2.2 Hz). |
| **Violent bounce and rollover when hitting curbs** | **Chassis Bottom-Out.** The suspension reaches its limit and the `CollisionShape3D` strikes the track. | 1. **Increase** `bump_stop_multiplier` (e.g., 4.0).<br>2. Raise the Y offset of the `CollisionShape3D`. |
| **Chronic oversteer (Spin-outs) when accelerating in corners** | **Excessive rear frequency (Rear too stiff).** The rear spring is too stiff relative to the front, losing mechanical grip at the rear. | 1. Increase `rear_resting_ratio` (soften spring).<br>2. Slightly lower the anti-roll bar `rear_arb_ratio`. |
| **Chronic understeer (The car won't turn)** | **Excessive front frequency (Nose too stiff).** | 1. Soften `front_resting_ratio`.<br>2. Shift more weight to the front axle (`front_weight_distribution`). |
| **The car oscillates up and down endlessly** | **Critical Damping is too low.** The springs lack oscillation braking. | Increase the `damping_ratio` on both axles (aim for 0.70 - 0.90). |
| **Wheels spin for no reason (Wheel Spin)** | **Insufficient longitudinal grip or excessive Torque.** | 1. Check if `peak_torque` in the V10 is realistic.<br>2. Check base tire friction or lower pressures. |

## 3. Direct Instruction for the Agent (V4 Pro / Flash)
Before proposing **any** changes to `f1_2026_car.tscn`:
1. Identify your symptom in the table above.
2. If in doubt, run the Python scripts in `tools/physics_diagnostics/` to obtain natural frequencies and damping ratios.
3. **NEVER ASSUME MAGIC:** In a physics engine (like GEVP), values represent Newton's mathematics. Do not invent multipliers.
