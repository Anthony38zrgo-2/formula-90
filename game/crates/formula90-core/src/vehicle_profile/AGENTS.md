# vehicle_profile — Complete profile coordination

Scope: coordinates physical configuration resolution (vehicle-physics-engine) with audio document loading (vehicle-audio-engine) into one complete vehicle profile.
Consumers: bootstrap, runtime and diagnostics that need both domains from a single manifest.
Rules: the physics crate never depends on the audio crate; this module owns the composition and the combined provenance digest.
