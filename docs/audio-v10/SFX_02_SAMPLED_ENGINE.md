# SFX-02 — Motor V10 100% sampleado (banco `v10_f2002_sampled`) + gearbox procedural

Date: 2026-09-19 · Branch: `main-clean` · HEAD: `70f10c166b1db4626b54ae430afc5cb1b2778bfa`
BUILD_SOURCE: `70f10c166b1db4626b54ae430afc5cb1b2778bfa` (rebuild release + debug)
Origen: pedido del usuario — banco únicamente sampleado a partir de los samples curados de
`implementation/sfx`, sin modificar sus RPM originales; el motor NO pasa por capas acústicas y
se valida que en el renderizado se suma con la capa procedural del gearbox. Conexión temporal
al f1_2030 para pruebas.

## Cadena final

```text
Banco V10 (sample layer: zonas ON/OFF, tonal+residual, gesto de cambio)
   │  drive = output × gesto × 0.61 × sampled_engine_gain
   ▼
SIN AcousticScene (sin dry split, comb aéreo, cover/mount, HP onboard ni output_gain)
   │
   ├─ + gearbox procedural = TransmissionSynth + one-shots gearup/geardn
   ▼
upper_mid_shelf (perfil, 0.0 = bit-transparente)
   ▼
mixer (headroom 0.62 → limitador → master)
```

El `AcousticScene` se construye al iniciar pero no se procesa en `SampleOnly`; la ruta híbrida
queda intacta (default `Hybrid`).

## Banco: exactamente los 11 archivos curados de `implementation/sfx`

El usuario depuró la carpeta fuente: solo quedan los archivos que deben usarse. Los 11 hashes
coinciden con el inventario original (`reports/audio-v10/sfx-integration/source_inventory.json`).
El banco de 11 fuentes (7 ON en 6 zonas + 4 OFF en 3 zonas) conserva las anclas medidas:

- ON (6 zonas): low_on 4579.5 · med_on 6366 · med_hi_on 6720 · almost_hi_on 7888.5 ·
  grupo [hi_max_on 8418 + hi_on 8469] · max_on 8952.
- OFF (3 zonas): low_off 3954 · grupo [hi_off 6216 + hi_off_2 6438] · max_off 8268.

`anchors.json:excluded` documenta los 11 descartes: 7 tomas de exterior por la auditoría
espectral (`exterior_spectral_audit.json`, cuerpo 40–160 Hz y órdenes 0.5–4 muy por debajo del
set interior) y 4 fuentes retiradas por el usuario (`idle`, `low_off_rear`, `med_off_rear`,
`med_off_front_near2`). Con el banco curado, el ralentí del f1_2030 (4500 rpm) lo cubre
`low_on` a rate 0.98.

Los one-shots de eventos (backfire/limiter/TC) ya no dependen de `implementation/sfx`: sus
fuentes se preservaron en `source-assets/audio/replacements/f1_2030/` (7 WAV) y el manifiesto
del banco `v10_vehicle` apunta allí; los WAV de runtime no cambiaron (mismos hashes).

## Cambios de código

- `runtime.rs::render_sampled_sample`: sin `EngineFrame` ni `scene.process`; salida
  `shelf(drive + gearbox × sampled_gearbox_gain)`.
- `v10_render.rs`: bypass de la escena en warmup y loop del modo sample-only; stems
  `sampled_engine`/`gearbox` explícitos; `scene_mix` = 0 en este modo.
- Perfil temporal `f1_2030_v10_geometric.json` (`audio.gf509`): `engine_mode: "sample_only"`,
  `sampled_engine_gain: 2.65`, `sampled_gearbox_gain: 1.0`, `upper_mid_shelf_gain: 0.0`,
  `sample_zone_trim_db: [-0.2, -1.3, -0.2, 0.1, 0.25, 1.3]`,
  `manifest: audio/v10_f2002_sampled/manifest.json`. Reversión:
  `reports/audio-v10/f2002-sampled/profile_backup_geometric_pre_sampled.json`.
- Builder `scripts/audio/build_v10_f2002_sampled_bank.py`: set curado de 11 fuentes, validación
  `conservados + excluidos = auditados` y limpieza de artefactos huérfanos (loop stems, metadata
  y `.import`).

## Corrección: caída de nivel en la mitad del sweep (6.7k rpm)

El análisis del sweep (`scratch/audio/f2002-sampled-audit/sweep_dip_analysis.py`) localizó una
caída de **−6.4 dB en 6721 rpm** (banda 40–300 Hz: −7.7 dB en ~6896 rpm). No era un loop con
seam defectuoso: los niveles crudos por zona medidos en steady están parejos (±1.3 dB). La causa
fue el array `sample_zone_trim_db`, que estaba escrito para la geometría de 7 zonas con `idle`
primero; al curar el banco a 11 fuentes (6 zonas ON) los trims quedaron desplazados un índice:

| Zona | Trim aplicado | Trim intencionado | Error |
|---|---:|---:|---:|
| low_on 4579.5 | 0.0 | — | — |
| med_on 6366 | **+4.5** | — | **+9.1 dB medido** |
| med_hi_on 6720 | **−4.63** | — | **−8.0 dB medido** |

Verificación por renders steady (`trimcheck_*_stems/sampled_engine.wav`): `med_on` −12.57 dB
con el índice viejo vs −21.70 re-mapeado; `med_hi_on` −22.84 vs −14.85. En lugar de reutilizar
los trims del híbrido (dejaban un escalón de 7 dB entre zonas), se midieron los niveles crudos
de las 6 zonas (dentro de ±1.3 dB) y se fijó un aplanado medido
`[-0.2, -1.3, -0.2, 0.1, 0.25, 1.3]`. Resultado tras el fix: el peor desvío en aceleración es
~−1.85 dB (antes −6.4 dB); el −2.4 dB en 8.3 s con −4.8 dB en graves es el lift físico
(off-throttle), no una discontinuidad de zona.

## Validación

| Puerta | Resultado |
|---|---|
| `v10-engine-synth` lib | 143 passed, 1 ignored (incluye bypass acústico y suma del gearbox) |
| `vehicle-audio-engine` lib | 234 passed |
| `formula90-core` lib | 30 passed |
| `tools/audio` pytest | 47 passed (2 fallos `retired_keys` preexistentes en HEAD) |
| **Identidad de suma** | shelf 0 + secuencia `u@2.5,d@7.2`: `max abs(mix − (sampled_engine + gearbox)) = 3.05e-05` (1 LSB PCM16) en 529,200 muestras; límite 5e-4 |
| **Capa de gearbox** | picos del stem `gearbox`: 0.046 en régimen, 0.086 upshift, 0.105 downshift |
| **Bypass acústico** | `scene.output_gain = 5` y ramas a 1.5 no alteran la salida sample-only |
| **Planitud del sweep** | peor desvío en aceleración −1.85 dB (antes −6.4 dB en 6721 rpm); el lift a 8.3 s muestra −2.4 dB con −4.8 dB en 40–300 Hz (off-throttle físico) |
| `aud_path_bench` (perfil real) | exit 0 sin fallback |
| `build_windows.ps1` release + debug | exit 0; `BUILD_SOURCE == HEAD` |
| `run_f1_94 -ValidateRuntimeOnly` / `-SmokeAudio` | PASS |
| Nivel | LUFS −16.35 vs híbrido −16.28; pico 0.652 (≈0.40 tras headroom 0.62) |

## Artefactos para validación humana

- `reports/audio-v10/f2002-sampled/render/sampled_sweep_5000_18000_lift_coast.wav`
  (5000→18000 rpm en 8 s + lift & coast a 5000 rpm; sin escena, shelf 0, trims aplanados,
  ganancia 2.65).
- `reports/audio-v10/f2002-sampled/render/hybrid_reference_sweep_5000_18000_lift_coast.wav`
  (referencia híbrida, misma trayectoria).
- `reports/audio-v10/f2002-sampled/render/auditions/` (normalizadas a −16.28 LUFS).
- `reports/audio-v10/f2002-sampled/render/sum_identity.wav` + `sum_identity_stems/`.
- Stems del sweep: `sampled_sweep_stems/` (`sampled_engine`, `gearbox`).

## Riesgos y pendientes

1. **Gate humano de escucha**: sin la escena, el timbre es más seco y con más graves que el
   híbrido; validar `sampled_engine_gain 2.65` y `sampled_gearbox_gain 1.0` y si se quiere
   HP/shelf de perfil (reactivable sin rebuild). Los trims quedan aplanados a nivel medido.
2. Con 11 fuentes, la zona ON más baja es 4579.5 y el hueco 4579→6366 se cubre por crossfade
   con pitch; no hay idle grabado (fuente retirada).
3. Cambios sin commit (a la espera de aprobación); el perfil temporal sigue apuntando al banco
   sampleado.

## Addendum — Tratamiento espectral V10/gearbox (sprint mixer)

El perfil pasa a `engine_mode: "hybrid"` (motor procedural + capa `v10_f2002_sampled`) y el
gearbox procedural sale de `AcousticScene` para recibir un tratamiento propio.

Cadenas configurables por perfil (`audio.gf509.v10_filter` / `audio.gf509.gearbox_filter`),
implementadas en `v10-engine-synth` (`tone.rs`: `SpectrumChain`, biquads en cascada Q=0.707;
72 dB/oct = 6 etapas de 12 dB/oct). Defaults = bypass bit-transparente.

- V10 (motor procedural + samples): HP 97 Hz 24 dB/oct → LP 14.41 kHz 72 dB/oct →
  peaking 10.0 kHz +3.0 dB Q 3.00 → distorsión tanh simétrica 2x-oversampleada sobre la
  banda del bell (`peak_drive` 1000, `peak_drive_mix` 0.3).
- Gearbox (TransmissionSynth + gearup/geardn): HP 515 Hz 72 dB/oct → LP 3.1 kHz 72 dB/oct.

Topología en híbrido (`runtime.rs::render_hybrid_sample`): `engine_frame` sin
`transmission`/`gear_shift` → `AcousticScene` → `blend_hybrid` con la capa sampleada →
`v10_filter`; gearbox crudo → `gearbox_filter`; `salida = upper_mid_shelf(v10 + gearbox)`.
El gearbox ya no recibe coloración de la escena. En sample-only se aplican las mismas cadenas
sobre `drive` y `gearbox`.

Paridad offline en `v10_render` (args `--v10-*` / `--gearbox-*`; stems `v10_filtered` y
`gearbox_filtered`).

Evidencia: `reports/audio-v10/f2002-sampled/filtered-sweep/` (`hybrid_v10_sweep_*.wav`,
`v10_sweep_*.wav`, `gearbox_sweep_*.wav`); identidad
`mix == v10_filtered + gearbox_filtered` = 1 LSB PCM16; el gearbox filtrado concentra 99.6%
de la energía en 515–3100 Hz. Calibración de la distorsión: la banda del bell está ~−33 dB
bajo la señal, así que `peak_drive` 2.0 resulta inaudible; con 1000 la distorsión añadida
queda en ~−48 dB (sutil) y por encima de ~1000 satura (meseta). Validación: v10-engine-synth
152 passed; vehicle-audio-engine 234 passed; formula90-core 31 passed; `build_windows.ps1
-Configuration release` y `run_f1_94 -ValidateRuntimeOnly` PASS.

