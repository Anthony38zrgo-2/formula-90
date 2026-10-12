# vehicle_configuration — Domain-split vehicle configuration

Scope: domain modules, the versioned manifest contract, the profile loader with deterministic provenance, the resolved transport document and the temporary legacy adapter.
Consumers: the physics crate, game-sim, formula90-core and offline diagnostics.
Rules: values, defaults and rounding move verbatim from the legacy monolith; behavior must stay identical until the domain migration completes. The loader never reads the audio or suspension presentation domains; formula90-core coordinates those owners.
