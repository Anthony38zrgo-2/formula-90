# Sound Mixer Configuration — Schema v2

## Overview

The `sound_mixer_config.json` file controls the vehicle audio runtime. It supports two schema versions:

- **Schema v1** (legacy): Simple per-sample gain map + exhaust behaviour
- **Schema v2** (full sound design): Per-sound DSP chains, reverb buses, master output, hot reload

The mixer loads this file from `game/sounds/sound_mixer_config.json`. Missing or invalid files degrade to safe defaults — the mixer never fails to start.

## Schema v2 Structure

```json
{
  "schema_version": 2,
  "defaults": { ... },
  "sounds": { "<bank_key>": { ... } },
  "reverb_buses": { "<bus_name>": { ... } },
  "exhaust": { ... },
  "master": { ... },
  "hot_reload": { ... }
}
```

### Per-Sound Configuration (`sounds.<key>`)

Each sound inherits from `defaults`; override any field to customize.

| Field | Type | Range | Default | Description |
|-------|------|-------|---------|-------------|
| `volume` | float | [0, 2] | 1.0 | Linear gain. Values >1 allow boost; master/headroom protects. |
| `pan` | float | [-1, 1] | 0.0 | Stereo position. Equal-power law: L=cos((pan+1)π/4), R=sin((pan+1)π/4). |
| `adsr` | object | — | disabled | Attack-Decay-Sustain-Release envelope. See ADSR section. |
| `eq` | object | — | disabled | 10-band graphic EQ with tube coloration. See EQ section. |
| `reverb` | object | — | disabled | Send to a shared reverb bus. See Reverb section. |

#### ADSR Envelope

| Field | Type | Range | Default | Description |
|-------|------|-------|---------|-------------|
| `enabled` | bool | — | false | Enable ADSR processing for this sound. |
| `attack_ms` | float | [0, 10000] | 0.0 | Attack time in milliseconds. |
| `decay_ms` | float | [0, 10000] | 0.0 | Decay time in milliseconds. |
| `sustain` | float | [0, 1] | 1.0 | Sustain level (0=silent, 1=full). |
| `release_ms` | float | [0, 10000] | 5.8 | Release time in milliseconds. |
| `curve` | string | linear/exponential/logarithmic | "linear" | Envelope shape. |

**One-shots**: note-on at trigger; release starts `release_ms` before sample end.
**Loops**: note-on at activate, note-off at deactivate; cursor continues during release.

#### Graphic EQ

| Field | Type | Range | Default | Description |
|-------|------|-------|---------|-------------|
| `enabled` | bool | — | false | Enable EQ processing. |
| `bands_db` | object | [-18, +18] dB | {} | Per-band gain. Keys: "31","62","125","250","500","1000","2000","4000","8000","16000". |
| `q` | float | [0.25, 4] | 1.0 | Filter Q factor (bandwidth). |
| `output_db` | float | [-24, +12] | 0.0 | Output trim after EQ. |
| `tube_color` | object | — | disabled | Valve warmth effect. See Tube Color section. |

#### Tube Color

| Field | Type | Range | Default | Description |
|-------|------|-------|---------|-------------|
| `enabled` | bool | — | false | Enable tube coloration. |
| `amount` | float | [0, 1] | 0.0 | Drive amount. |
| `boost_link` | float | [0, 1] | 0.35 | Link drive to positive EQ boosts. |
| `bias` | float | [0, 0.5] | 0.05 | DC bias offset. |
| `mix` | float | [0, 1] | 0.15 | Dry/wet blend. |
| `auto_gain` | bool | — | true | Automatic gain compensation. |

**Drive formula**: `effective_drive = amount + boost_link * sum(positive_boosts)`
Cuts do not increase drive. Bypass is transparent when disabled.

#### Reverb Send

| Field | Type | Range | Default | Description |
|-------|------|-------|---------|-------------|
| `enabled` | bool | — | false | Enable reverb send. |
| `bus` | string | — | "" | Target reverb bus name (must exist in `reverb_buses`). |
| `send_db` | float | [-120, +6] | -120.0 | Send level in dB. -120 dB = silence/bypass. |

The send tap is post-ADSR, post-EQ, post-volume, post-pan. The reverb receives the designed character and preserves the SFX position in early reflections.

### Reverb Buses (`reverb_buses.<name>`)

Shared stereo reverb processors. Multiple sounds can send to the same bus.

| Field | Type | Range | Default | Description |
|-------|------|-------|---------|-------------|
| `enabled` | bool | — | true | Enable this bus. |
| `pre_delay_ms` | float | [0, 250] | 8.0 | Pre-delay before reverb onset. |
| `decay_s` | float | [0.05, 10] | 0.55 | RT60 decay time in seconds. |
| `damping` | float | [0, 1] | 0.58 | High-frequency damping. |
| `low_cut_hz` | float | [20, 2000] | 180.0 | Low-cut filter frequency. |
| `high_cut_hz` | float | [1000, 20000] | 6800.0 | High-cut filter frequency (must be > low_cut_hz). |
| `stereo_width` | float | [0, 1] | 0.75 | Stereo decorrelation. 0=mono, 1=full stereo. |
| `wet_db` | float | [-120, +6] | -3.0 | Wet output level. |

**Recommended presets**: `impact_short`, `mechanical_small`, `exhaust_chamber`, `surface_ambience`.

### Master Output

| Field | Type | Range | Default | Description |
|-------|------|-------|---------|-------------|
| `output_db` | float | [-24, +12] | 0.0 | Master output trim. |
| `limiter_threshold` | float | [0, 1] | 0.90 | Limiter ceiling. No frame exceeds this. |
| `saturation` | float | [0, 1] | 0.12 | Soft saturation amount. |

### Hot Reload

| Field | Type | Range | Default | Description |
|-------|------|-------|---------|-------------|
| `enabled` | bool | — | false | Enable file watching. |
| `poll_interval_ms` | u32 | [50, 5000] | 250 | File check interval. |
| `transition_ms` | u32 | [0, 500] | 30 | Crossfade duration on reload. |

Changes are applied between render blocks. Errors preserve the last valid snapshot.

## Validation & Safety

All values are validated on load:
- Non-finite values (NaN, Inf) → replaced with safe defaults
- Out-of-range values → clamped to valid range
- Invalid EQ frequencies → removed (only ISO centers allowed)
- Missing reverb bus references → error logged, send disabled
- Unknown keys → warning logged, ignored

Run `va_validate <config.json> --check` to verify before deployment.

## Migration from v1

Schema v1 configs are automatically migrated internally:
- `gains.<key>` → `sounds.<key>.volume`
- All DSP features default to disabled (bypass)
- Output is bit-identical to v1 when DSP is bypassed

Use `va_validate old.json --migrate --output new.json` to convert explicitly.

## CLI Tools

### va_baseline
Render deterministic scenarios and capture golden metrics for regression testing.
```
cargo run -p vehicle_audio_engine --bin va_baseline -- game/sounds/banks/v10_vehicle
```

### va_validate
Validate, summarize, and migrate configuration files.
```
va_validate game/sounds/sound_mixer_config.json           # Summary
va_validate game/sounds/sound_mixer_config.json --check   # Validate only
va_validate old.json --migrate --output new.json          # Migrate v1→v2
```

## Signal Chain

```
SOURCE (mono)
  → ADSR envelope
  → Graphic EQ (10 bands)
  → Tube coloration
  → Volume [0,2]
  → Pan equal-power [-1,1]
  ├→ Dry mix L/R
  └→ Reverb send → Shared bus → Return L/R
                                    ↓
                      Sum dry + returns
                                    ↓
                Finite/DC protection + master gain
                                    ↓
                  Stereo linked limiter → ABI output
```

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| Sound unchanged after edit | Hot reload disabled | Set `hot_reload.enabled: true` |
| Click/pop on parameter change | Transition too fast | Increase `hot_reload.transition_ms` |
| Distortion/clipping | Excessive boost accumulation | Reduce EQ/output_db or volume |
| Reverb tail cut off | Bus disabled or decay too short | Check `reverb_buses.<name>.enabled` and `decay_s` |
| Mono output despite pan | DSP chain not active | Ensure `schema_version: 2` and relevant sections enabled |
| Config ignored on load | JSON syntax error | Run `va_validate --check` for diagnostics |
