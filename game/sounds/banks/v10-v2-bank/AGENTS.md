# v10-v2-bank — Canonical V10 engine bank

Scope: the canonical runtime V10 engine bank: manifest, source inventory, raw sources, and derived loops and events co-located.
Consumers: the F1 2030 V10 vehicle profile and the runtime sampler.
Rules: raw names (`engine_*`, `backfire_*`, `gearup`, `geardn`, `limiter`) are sources; derived names (`*_loop`, `*_event`, `backfire_burst_*`) are runtime assets. Rebuild only with `tools/audio/build_canonical_v10_engine_bank.py`; verify with `--check`. This folder carries a `.gdignore`: Godot never imports these WAVs.

`limiter.wav` preserves the supplied limiter recording. `limiter_cut_loop.wav` is its mono16 44100 Hz runtime derivative. The limiter group uses `limiter_cut_window`: playback follows the physical limiter cut intervals, with no entry event, cooldown, or audio-side revolutions threshold. Other groups retain their event behavior.
