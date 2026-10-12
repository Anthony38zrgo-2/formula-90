# vehicle_configuration — Domain-split vehicle configuration

Scope: domain modules and the temporary legacy adapter for vehicle configuration.
Consumers: the physics crate, game-sim and formula90-core.
Rules: values, defaults and rounding move verbatim from the legacy monolith; behavior must stay identical until the domain migration completes.
