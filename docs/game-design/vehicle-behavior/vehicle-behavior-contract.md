# Vehicle Behavior Contract

## Philosophy
This project (F1 2030) aims for an **Arcade/Convincing** driving experience, not a pure motorsport simulation. 

## High-Level Contract
- **Steering:** Progressive. No instant grip response.
- **Weight Transfer:** Must be understandable to the player.
- **Oversteer:** Must be recoverable when corrected appropriately.
- **Understeer:** Must be understandable (e.g. entering a corner too fast).
- **Braking:** Progressive behavior, trailing should induce slight oversteer.
- **Visual Consequences:** Late corrections should have visible consequences (body roll, slide).

## Rule of Engagement
Do NOT turn the project into a high-fidelity motorsport simulator unless explicitly requested. The purpose of mathematical rigor is to make the arcade behavior predictable and controllable, not to maximize simulation complexity.
