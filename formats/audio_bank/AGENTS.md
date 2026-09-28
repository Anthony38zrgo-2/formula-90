# audio_bank — Bank manifest contract

Scope: the versioned manifest schema of a built runtime sound bank and its companion contract notes.
Consumers: `tools/audio/` producers, the vehicle audio Rust crates, and the banks under `game/sounds/banks/`.
Rules: the executable contract is `manifest.schema.json`. Adding an optional field keeps the version; making a field required or changing meaning mints a new version with a coordinated cutover.
