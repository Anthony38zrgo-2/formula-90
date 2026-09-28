# src/core — Core implementations

Scope: bootstrap, shared runtime state, and Godot module registration.
Consumers: every library loaded through `game/addons/formula90s/`.
Rules: registration order is load-bearing; adding a module touches registration and its test together.
