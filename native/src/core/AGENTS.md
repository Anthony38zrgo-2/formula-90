# src/core — Core implementations

Scope: bootstrap, shared runtime state, and Godot module registration.
Consumers: every library loaded through `game/addons/formula90s/`.
Rules: registration order is load-bearing; adding a module touches registration and its test together.

The physical-world loader validates interface capabilities and full source parity before accepting a library. Destroy owned worlds before unloading. Serialize requests at full precision with exact integral-value normalization.

The core snapshot adapter reuses telemetry and audio publication without stepping the shadow simulation. Keep physical ownership checks on legacy callbacks and clear cached vehicle references on removal. A loader rejection must not leave an active world using unloaded code.
