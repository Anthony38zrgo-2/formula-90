# v10-bank-ai — AI-only V10 engine bank

Scope: the runtime V10 engine bank for artificial drivers only: manifest,
source inventory, and prepared runtime assets co-located. Engine loops only:
no transmission whine, no gearbox events, no backfire, no limiter.
Consumers: artificial-driver voices under the `AI_ENGINE_ONLY` policy (max
two audible AI voices alongside the player). The player keeps the canonical
`v10-v2-bank` untouched.
Rules: this bank coexists with the canonical bank and never supersedes it.
Raw recordings live in `game/sounds/bank_sources/v10-bank-ai/` with optional
per-sample sidecar `v10_ai_source_inventory.json` for authored revolutions
and individual crossfade widths. Rebuild only with
`python -m tools.audio.build_ai_v10_engine_bank --promote` in offline mode.
Never hand-edit prepared WAVs, `manifest.json` or `source_inventory.json`.
This folder carries a `.gdignore`: Godot never imports these WAVs.

The bank holds six powered loops (idle plus five on-throttle layers) and four
coast layers covering 4500 to 18000 revolutions per minute. Powered presence
uses smoothstep equal-power crossfades derived from per-sample widths;
adjacent zones closer than eight percent use the narrow width. All runtime
loops use mono PCM16 at 44100 Hz with integer engine-cycle phase coherence
and equal-power wrap crossfades.
