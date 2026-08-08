# Known Issues & Engineering Memory

This document serves as the long-term engineering memory for the F1 2030 project.

## 1. Physics & Handling
- **Excessive Damping Trap:** Do not use `damping_ratio > 1.0` to compensate for mass distribution. GEVP calculates force internally using mass * stiffness. 
- **Curb Geometry:** Rigid 90-degree curbs in Godot 4 CSGPolygons create infinite horizontal force spikes for raycast vehicles. Curbs must have sloped chamfers.

## 2. Tooling & Parsers
- **`tscn_parser.py` Key Mismatches:** Godot `.tscn` files often export variables differently from how GDScript internal variables are named (e.g. `rear_bump_stop_multiplier` vs `rear_bump_stop_mult`). Always check for string mismatches if a parser returns default values incorrectly.
