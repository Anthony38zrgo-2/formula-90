# REF-001 Phase 7 — engine audio SRP extraction

## Scope

Extract sample-layer playback and DSP mixing from `EngineAudioController`
without changing audio assets, scene resources, exported properties, or
vehicle behavior.

## Ownership after the change

- `EngineAudioMixer` owns PCM layer data and cursors, loop interpolation,
  engine-layer crossfade, gain smoothing, gear-shift sample playback,
  rev-cut modulation, and the DSP chain.
- `EngineAudioController` owns Godot integration: WAV resource loading,
  vehicle-state reads, `AudioStreamGenerator` and `AudioStreamPlayer3D`
  lifetime, buffer delivery, exported configuration, and the gear-shift
  signal.
- `EngineAudioMixConfig` passes only the scalar configuration needed for a
  frame of mixing. The mixer does not resolve Godot nodes or resources.

## Behavior preservation

- PCM16/mono and 44.1/48 kHz validation remains in the Godot resource loader.
- The generator still uses the rate of the final successfully loaded sample,
  as before.
- Gain attack/release, layer weighting, pitch, shift gain, rev-cut cadence,
  saturation, and limiter calculations were moved without retuning.
- No engine bank, `.tres` resource, scene, or vehicle property changed.

## Validation

- `powershell -ExecutionPolicy Bypass -File scripts/build_windows.ps1` passed
  and compiled `engine_audio_controller.cpp` and `engine_audio_mixer.cpp`.
- Headless editor import followed by `test_field.tscn` and `player_car.tscn`
  exited 0 and loaded the GDExtension.
- The output retains two established baseline problems: `test_field.tscn`
  instantiates `EngineAudioController` without `EngineAudioConfig`, and
  `player_car.tscn` references ignored Jordan 191 visual GLBs. Those prevent a
  clean end-to-end audible playback assertion in this isolated worktree.
- `scripts/test_windows.ps1` remains blocked before smoke tests by the
  pre-existing missing `formula90s/vehicle/physics_math.hpp` include.
