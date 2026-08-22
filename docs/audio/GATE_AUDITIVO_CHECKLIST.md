# GATE AUDITIVO HUMANO — Checklist y evidencia (2026-08-21)

Mapa de los criterios de finalización del handoff (§10) a artefactos
verificables, para la sesión de escucha. El backend `common_v10_commands`
sigue **opt-in** (solo la escena de prueba); `legacy_v10_pcm` sigue siendo el
default seguro y NO se ha borrado nada del fallback.

## 0. Estado del software (todo verificado automáticamente)

| artefacto | estado | verificación |
|---|---|---|
| `cargo test -p vehicle_audio_engine -p formula90_core` | 14 bins verdes, 74+ tests | `tests/` incl. 5 gates golden, A/B, colisiones, mutes, reset |
| Golden trace (`diagnostics/golden_trace_v10_v2.json`) | int on -5.7/off -6.8 dB, ext bed -6.1 dB, transmission 5-350 sin huecos | `tests/golden_trace.rs` + 4 sweeps |
| DLLs desplegadas | `formula90_core.dll` + template_release + GDExtension release/debug al día | timestamps posteriores a las fuentes |
| Catálogo v2 | `game/sounds/runtime/catalog.v2.json` (12 int, 24 ext, 3 trans + loops) | generado del dump del banco |
| A/B escenarios | `diagnostics/ab/{idle_to_redline,road_to_sand,grass_skid,impact}.{commands.json,legacy.wav}` | mismo escenario, ambos backends |
| A/B sesión real | `diagnostics/telemetry_live.csv` (2761 ticks) → `diagnostics/ab/live_telemetry_live.{commands.json,legacy.wav}` | 0 huecos, 33 voces pico |
| Captura por comando | renderer `debug_capture_path` + mutes `debug_mute_mask` | escenas de diagnóstico |

## 1. Sesión de escucha (orden sugerido, 15-20 min)

1. **Sweep interior** (`vehicle_test_session_engine_int.tscn`, mutes=254):
   conducción con cámara cockpit; solo suena engine_int.
   - Ralentí audible y continuo; subida 2k→17.4k sin huecos ni cortes;
     lift-off con capas off bajando de régimen.
   - Señalar si el **timbre/pitch** de alguna capa no corresponde
     (los `auto_pitch_ref` del catálogo permiten probar, capa a capa, la
     curva de automación + autopitch).
2. **A/B escenarios**: escuchar `diagnostics/ab/*.legacy.wav` (referencia
   legacy) y repetir la misma maniobra en `vehicle_test_session_full.tscn`
   (audio commands): cambios deben coincidir, superficies e impactos
   presentes, sin clicks evidentes de loop en capas dominantes.
3. **Sesión real**: `vehicle_test_session_record.tscn` graba la telemetría;
   tras la sesión, `cargo run -p vehicle_audio_engine --example live_replay
   -- <csv>` produce el par A/B de TU conducción.
4. **Familia a familia** (opcional): activar mutes progresivamente
   (254→126→94→78...) para auditar cada sistema.

## 2. Criterios del handoff §10 — evidencia y decisión

| criterio | evidencia automática | decisión humana |
|---|---|---|
| Samples correctos en orden y rango | catalog.v2 + ventanas exactas + golden 0 huecos | timbre/pitch por capa |
| Loops/transiciones sin gaps/clicks | `diagnostics/loop_seam_audit_final.json` + `loop_regions.json` (3 políticas por sample) | **decisión pendiente**: si un loop clickea en mezcla, aplicar política alternativa (corta/whole) o re-export `*.loop.wav` |
| Misma telemetría → misma secuencia | determinismo golden + A/B + seq | — |
| Sin productor legacy duplicado | greps escenas + smoke (28 voces, 1 renderer) | confirmar en la escucha |
| Motor interior/exterior, transmisión, superficies, impactos A/B | harness A/B + sweeps | aprobar cada familia |
| Gate auditivo aprobado | — | **este checklist** |
| Retrospectiva antes de deprecar | `docs/audio/RETROSPECTIVA_INTEGRACION_AUDIO.md` (borrador) | firmar/ajustar |
| 2 sprints estables antes de deprecar legacy | — | calendario |

## 3. Decisiones abiertas (solo el gate las cierra)

1. **Trims sin dato de banco**: wind -8 dB, wheel -12 dB (base del banco
   `tyre_rolling` es -12 ✓; wind no tiene instrumento con gain). A/B con y
   sin trim.
2. **Corte de 3 m interior/exterior**: confirmar transición cockpit↔chase cam
   (listener_distance); a 10 m el interior está mudo por diseño (verificado).
3. **Loops**: política por capa según `loop_regions.json` (whole/región
   larga/región corta); medición descarta seams ≤ -30 dB estáticos.
4. **`auto_pitch_ref`** por capa: pendiente de prueba auditiva (catálogo).

## 4. Después del gate (protocolo)

1. Confirmar el checklist → pasar `AudioBackend::CommonV10Commands` a default
   (nodo F90Core) en un sprint.
2. Dos sprints estables sin duplicados/gaps/regresiones → retrospectiva
   formal firmada.
3. Deprecar: `LegacyV10Pcm`, `v10_vehicle`, ABI separada, controllers nativos
   legacy (por orden, sin borrar hasta el paso 2).