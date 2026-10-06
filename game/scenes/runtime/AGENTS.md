# runtime — Session runtime scenes

Scope: race and vehicle test sessions, the world HUD compositor, and camera rigs.
Consumers: every playable and diagnostic run of the game.
Rules: session scenes compose systems from `game/scripts/runtime/`; cross-scene contracts go through session configuration.

Both F1 2030 session variants contain an inactive physical-world controller with explicit player vehicle, presentation core and profile bindings. Keep it inactive by default. Activation requires a validated physical package through the canonical launcher. Adding physical vehicles requires matched bindings and a single Rust world owner; Godot proxies must not also integrate or resolve collisions.
