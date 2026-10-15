# race_intelligence — Artificial driver tuning data

Scope: load-time tuning for artificial drivers: automatic transmission
shift logic, solitary driver control, audio voice policy, and physics profile selection.
Consumers: `race-intelligence-engine`, the Godot bridge adapter, and offline
diagnostics.
Rules: data-only tuning truth; code defaults never override these files
silently. The player vehicle profile stays manual; only artificial drivers
select automatic transmission through these files.

Both driver tuning documents declare nominal grip reserves, recovery slew,
speed tracking band, and separate throttle/brake actuator rates. The unrestricted
document uses a null global speed cap; the vehicle-derived profile remains
responsible for cornering and braking constraints.
