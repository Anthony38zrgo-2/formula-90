# runtime — Session runtime scenes

Scope: race and vehicle test sessions, the world HUD compositor, and camera rigs.
Consumers: every playable and diagnostic run of the game.
Rules: session scenes compose systems from `game/scripts/runtime/`; cross-scene contracts go through session configuration.
