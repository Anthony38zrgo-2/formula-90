# vehicle-physics-engine — Runtime vehicle dynamics

Scope: the vehicle dynamics engine behind `vehicle_physics_engine.dll`.
Consumers: the bridge vehicle adapter and vehicle scenes.
Rules: physics authority lives here. Tunables arrive from `game/data/vehicles/`; constants require a data change, not a code fork.
Subfolders: each child folder documents itself in its own AGENTS.md.
