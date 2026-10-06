# runtime — Session runtime GDScript

Scope: lap timing, pit-stop rules and visuals, and session configuration.
Consumers: race session scenes.
Rules: rules scripts decide, visual scripts show, configuration composes. Never mix the three.

## Optional physical world

`physical_vehicle_world_controller.gd` requires explicit package, profile, vehicle and presentation-core bindings. It remains disabled by default. Each vehicle requires its own matched core; reject unregistered moving Godot collision bodies.

Queue all vehicle inputs before advancing the shared Rust world once per host physics tick. Preserve physical timestamps, unconsumed host fractions, one-shot gear requests and pause behavior. Route reset, refuel and tire replacement to the world, then republish authoritative snapshots. Removal clears the entity and cached presentation reference.

Freeze native vehicle proxies and disable their Godot collision layers, gravity and legacy force callbacks under physical ownership. Failure stops scheduling and freezes the bound bodies. Impact events can feed existing impact audio with its cooldown; continuous scraping remains pending.
