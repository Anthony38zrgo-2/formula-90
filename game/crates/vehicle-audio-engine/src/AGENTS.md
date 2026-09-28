# src — Audio engine implementation

Scope: bank loading, DSP chain, synth playback, and runner binaries.
Consumers: the bridge audio adapter.
Rules: real-time safety: no allocation or I/O on the audio path. The `bin/` headless-runner folder is ignore-blocked from carrying its own file and is covered here: runners fail loudly on manifest mismatch, never fall back to a default bank.
