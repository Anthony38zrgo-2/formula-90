# sounds — Runtime audio content

Scope: bank sources, runtime banks, and the mixer policy. This folder owns
the audio canon stated below; each bank documents itself in its own file.
Consumers: the vehicle audio engine loading banks from the filesystem, never
through `res://` paths.
Rules: runtime banks live only under `banks/`. Pure binary source leaves
carry no AGENTS.md and are covered here. Subfolders: each bank documents
itself in its own AGENTS.md.

## Sound bank canon

- The canonical V10 engine bank is `game/sounds/banks/v10-v2-bank`.
- The V10 engine bank and the shared commons bank are the runtime banks: never
  create new runtime banks under `game/audio/`; that tree is historical.
  Shipped sessions never load it; the runtime reaches into it only for the
  alternative `gf509` diagnostic source declared through
  `audio.continuous_source = "v10_gf509"` in vehicle profiles.
- The shared commons bank stays at `game/sounds/banks/commons` because it is not
  V10-specific. Do not move it into `v10-v2-bank` and do not rename its
  `bank_manifest.json`.
- Runtime assets declared by `v10-v2-bank/manifest.json` must sit next to
  that manifest inside `v10-v2-bank`. The bank ships prepared runtime WAVs
  only: derived file names (`*_loop.wav`, `*_event.wav`, `backfire_burst_*`,
  `limiter_cut_loop.wav`, `transmission_whine_loop.wav`) are runtime assets.
  Raw recordings stay outside the bank; stage new ones nowhere inside
  `banks/` and finish promotion offline before updating the manifest.
- The geometric F1 2030 vehicle profile selects the canonical V10 bank through
  `audio.grand_prix_sampler.manifest = "sounds/banks/v10-v2-bank/manifest.json"`
  in `game/data/vehicles/f1_2030/profiles/geometric/audio.json`.
  The runtime resolves this path relative to the Godot project root.
- `banks/v8-bank` is an inactive Ferrari candidate retained for later comparison.
  Regenerate it with `tools/audio/build_ferrari_recorded_engine_bank.py`.
  Its metadata explicitly documents reconstructed interpolation and perspective limits.
- `bank_sources/` and `banks/v10-v2-bank/` each carry a `.gdignore`: Godot must
  not import those WAVs. The Rust runtime loads bank audio from the filesystem,
  so bank assets are never referenced through `res://` paths.

## Sampling format

Sampling format truth and engine-bank manifest invariants live in
`formats/audio_bank/`; the loader rejects any mismatch (`read_wav_mono16` in
`game/crates/vehicle-audio-engine/src/bank.rs`).

## Regeneration rules

- Rebuild the canon only with
  `python -m tools.audio.build_canonical_v10_engine_bank` from the repository
  root in offline mode. Run
  `python -m tools.audio.build_canonical_v10_engine_bank --check` to validate
  the canonical provenance records without rewriting anything, and `--audit`
  to list which recorded sources remain resolvable on disk. Runtime WAVs keep
  preparation provenance inside `source_inventory.json`.
- Never hand-edit prepared runtime WAVs, `manifest.json` or
  `source_inventory.json`. Any intentional audio change updates recorded
  sources offline, then the tool revision, derived hashes and the tests in
  `tools/audio/tests/test_canonical_v10_engine_bank.py` in the same change.
- The preparation tool is deterministic. It never rewrites runtime WAVs.
- New recordings are promoted offline outside the bank; the bank never carries
  raw capture sessions.
