# vehicle_audio_configuration — Audio profile documents

Scope: the `audio` domain document (continuous source, C++ DSP layer, GF509 layer, Grand Prix sampler, powertrain synthesis), its loader and validation.
Consumers: formula90-core profile coordination, which composes the complete vehicle profile.
Rules: the physics crate never parses this domain. GF509 stays a JSON object with a closed top-level key set until its nested schema is typed; powertrain synthesis validation delegates to the existing powertrain parser.
