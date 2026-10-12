# legacy — F1 2030 legacy profile manifest

Scope: the versioned domain documents split from f1_2030_v10_physics.json with every value preserved.
Consumers: physics manifest loading and formula90-core complete-profile composition.
Rules: this root is the editable source for the legacy profile; the monolith f1_2030_v10_physics.json stays frozen as temporary compatibility and must not diverge. The profile carries no coupled world, mass inventory or underfloor contact; suspension_presentation.json keeps the visual suspension geometry for presentation consumers and audio.json is owned by vehicle-audio-engine.
