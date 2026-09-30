# vehicle-audio-engine — Runtime vehicle audio

Scope: the runtime sampler engine behind `vehicle_audio_engine.dll`: bank loading, DSP, and synthesis playback.
Consumers: the Godot bridge and vehicle scenes.
Rules: format invariants in `formats/audio_bank/AGENTS.md` are load-bearing here; the loader rejects mismatched WAVs.
Subfolders: each child folder documents itself in its own AGENTS.md.
