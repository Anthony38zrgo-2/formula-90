# Retrospectiva — Integración audio Common + V10_V2 (BORRADOR, pendiente de gates)

Estado: borrador para la retrospectiva formal del protocolo
(sprint planning -> backlog -> implement -> review -> human gate ->
sprint retrospective -> done). **No es válida como cierre**: el gate auditivo
humano sigue pendiente y el backend `common_v10_commands` sigue opt-in.

## Qué pasó (hechos)

- El handoff (2026-08-21) declaró la feature en REVIEW/HUMAN GATE FALLIDO: el
  usuario confirmó que el audio nuevo sonaba mal aunque compilaba, los tests
  estructurales pasaban y Godot creaba reproductores.
- Diagnóstico con evidencia (esta iteración):
  1. El parser del `.bank` existía pero el dump no atribuía curvas a
     instrumentos; la cadena controller->property->instrumento se resolvió
     extendiendo el dump (contadores/property/curve) y verificando que los
     controllers apuntan al instrumento como property owner.
  2. Las ventanas RPM aproximadas de `commands.rs` divergían de las del banco:
     ralentí mudo a 2053 rpm (-71.5 dB), lift-off 15.6k-17.4k muerto (42
     huecos en sweep off), ventanas de capas on/off con rangos incorrectos.
  3. La telemetría usaba proxies: `tc_slip_ratio` (0 con TC off), velocidad de
     chasis por `drivetrain_speed`, colisiones que entregaban solo el primer
     contacto con cooldown y sin clasificación Vehicle.
  4. Los loops eran de archivo completo sin regiones verificadas (seams de
     0.3..5.5 dB) y las regiones hardcodeadas de 4 WAV no estaban alineadas al
     periodo de disparo.
  5. Un one-shot rechazado por el presupuesto se perdía para siempre (id ya
     consumido); los one-shots de 120 ticks rejugados se deduplicaban bien pero
     el fallo del presupuesto era silencioso.
- Correcciones aplicadas en orden (Fases 0-6): instrumentación y mutes por
  familia; catálogo v2 con tablas exactas (12+24+3 instrumentos); ventanas y
  gains exactos de engine_int (0 huecos, on -5.7/off -6.8 dB) y engine_ext
  (bed -6.1 dB); drivetrain_speed real; colisiones completas
  (top-4 ordenadas, kind Vehicle); slip real por rueda; harness A/B offline y
  grabador de sesión real (replay 2761 ticks, 0 huecos).

## Qué funcionó

- La restricción de no borrar el fallback permitió A/B en todo momento.
- El golden trace determinista convirtió el "suena mal" en números accionables
  (huecos, cobertura, determinismo) y quedó como gate de regresión.
- Mutes por familia: el gate humano puede aislar un sistema sin tocar código.
- Medir antes de tocar (seams, f0, periodos) evitó "arreglos" especulativos.

## Qué falló (lecciones de proceso)

1. **Continuidad perceptual sin gate de fidelidad**: los sprints se marcaron
   [x] con criterios estructurales ("compila", "smoke 14 players"), no con
   criterios sonoros. El backlog ya decía "resolver curvas hasta propiedad
   nombrada: [ ]" mientras los diccionarios declaraban el slice
   "implementado". **Lección**: un ítem de fidelidad no está hecho hasta tener
   la evidencia medida + gate auditivo.
2. **Tablas "recuperadas" sin cadena de propiedad**: se copiaron regiones de
   curvas sin resolver dueño->propiedad->parámetro; varias no pertenecían a la
   capa que se les atribuyó. **Lección**: registrar siempre la cadena completa
   (GUIDs) como hizo `catalog.v2.json`.
3. **Tests que no escuchan**: 64 tests estructurales no detectan clicks,
   huecos o capas ausentes. **Lección**: thresholds numéricos de cobertura y
   seam como complemento obligatorio.
4. **Binarización legacy/nuevo sin arnés A/B**: no existía forma de comparar
   ambos backends con la misma telemetría. **Lección**: el harness A/B (ahora
   en `tests/ab_legacy_commands.rs` + `examples/live_replay.rs`) debe existir
   desde el primer sprint de fidelidad.

## Decisiones tomadas

- Backend commands sigue opt-in (default legacy seguro) hasta el gate.
- Presupuesto de voces 32->48 (el banco tiene 55 instrumentos).
- Trims manuales conservados solo donde no hay dato de banco (wind -8),
  retirados donde el banco define el gain (transmission, floor_scrape).
- Loops: regiones alineadas al periodo solo donde el dato es claro; el resto
  de políticas queda en `loop_regions.json` (whole/region/short por sample)
  para la decisión auditiva. El horneado de crossfades quedó descartado por
  medición: el drift de las grabaciones impide seams <= -30 dB con regiones
  estáticas; la decisión final es del gate (enmascaramiento en mezcla).

## Antes de deprecar legacy (checklist pendiente)

1. [ ] Gate auditivo humano: sweep interior (escena `engine_int`), A/B WAVs
   (`diagnostics/ab/*`), escena completa (`vehicle_test_session_full`).
2. [ ] Decisión de loops por capa según `loop_regions.json`.
3. [ ] 2 sprints estables con `common_v10_commands` sin duplicados, gaps,
   regresiones de CPU o replay.
4. [ ] Retrospectiva formal firmada por el usuario.
5. [ ] Deprecación ordenada: `AudioBackend::LegacyV10Pcm` default -> commands,
   retirar `v10_vehicle`, ABI separada y controllers nativos legacy.

Referencia: `AUDIO_BANKS_INTEGRATION_BACKLOG.md` (registros Fase 0-6),
`FMOD_V10_V2_BANK_DICTIONARY.md`, `loop_seam_audit_final.json`,
`golden_trace_v10_v2.json`, `diagnostics/ab/`.