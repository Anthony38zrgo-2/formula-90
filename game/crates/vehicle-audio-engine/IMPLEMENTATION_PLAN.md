# Plan de Implementación — Sound Design Runtime

## Progreso Actual

### ✅ Sprint 0: Contrato y Baseline (Épico A) — COMPLETADO

| Tarea | Estado | Entregable |
|-------|--------|------------|
| A1 Baseline determinista | ✅ | `tests/baseline_golden.json`, `bin/va_baseline.rs` |
| A2 Tipos schema v2 | ✅ | `config.rs` (~900 líneas), 48 tests passing |
| A3 Validador estricto | ✅ | `bin/va_validate.rs` con --check, diagnósticos |
| A4 Compatibilidad v1→v2 | ✅ | `to_v2()`, migración probada |
| A5 Documentación | ✅ | `SOUND_CONFIG_DOCS.md` |

**Gate del contrato JSON/ADSR**: ✓ Superado
- Schema v1 cargado produce salida idéntica (L==R confirmado en baseline)
- Schema v2 parsea, valida y sanitiza correctamente
- Migración v1→v2 preserva gains originales
- Todos los valores no-finitos y fuera-de-rango manejados sin panic

---

### ⏳ Sprint 1: DSP Aislado (Épico B) — SIGUIENTE

| Tarea | Pts | Prioridad | Descripción |
|-------|-----|-----------|-------------|
| B1 ADSR sample-accurate | 5 | P0 | Estados Idle/Attack/Decay/Sustain/Release, coeficientes precalculados |
| B2 Biquad + EQ gráfico | 5 | P0 | Estado por voz, bypass 0dB, denormal protection, suavizado |
| B3 Paneo equal-power | 3 | P0 | L=cos((pan+1)π/4), R=sin((pan+1)π/4), potencia constante |
| B4 Color valvular | 5 | P0 | Parámetros JSON `enabled`, `amount`, `boost_link`, `bias`, `mix` y `auto_gain`; bypass transparente; THD/DC/aliasing medidos |
| B5 Buses reverb estéreo | 8 | P0 | FDN/Schroeder, pre-delay, damping, filtros, width, tail mgmt |
| B6 Limitador master estéreo | 3 | P0 | Techo configurable, enlazado L/R, no finitos |
| B7 Benchmark peor caso | 3 | P1 | <10% CPU core @ 44.1kHz, cero allocations en render |

**Gate**: EQ/color/reverb medido y auditivo

#### Ruta Crítica Sprint 1
```
B1 (ADSR) → B2 (EQ) → B3 (Pan) → B4 (Tube) → B5 (Reverb) → B6 (Limiter) → B7 (Benchmark)
```

#### Arquitectura de Módulos DSP
```
src/dsp/
├── mod.rs          # Re-exports
├── adsr.rs         # Envelope generator (state machine)
├── biquad.rs       # Second-order filter section
├── eq.rs           # 10-band graphic EQ (cascade of biquads)
├── pan.rs          # Equal-power stereo panner
├── tube.rs         # Valve coloration (soft clipper + bias)
├── reverb.rs       # FDN stereo reverb engine
└── limiter.rs      # Stereo linked peak limiter
```

#### Contrato de Tube Color

- Configurable por sonido mediante `eq.tube_color`: `enabled`, `amount`,
  `boost_link`, `bias`, `mix` y `auto_gain`.
- `enabled: false` debe hacer bypass transparente sin alterar nivel, fase ni DC,
  con tolerancia explícita y test automatizado.
- `mix: 0.0` debe conservar la ruta dry; `amount: 0.0` no debe introducir color
  aunque el bloque esté habilitado.
- Los parámetros se preparan fuera de `render()` y sus cambios usan suavizado o
  crossfade para evitar clicks.
- Los tests deben demostrar tanto cambio medible cuando está activo como
  transparencia cuando está en bypass.

---

### ⏸ Sprint 2: Integración Mixer (Épicos C+E)
- C1 VoiceStrip compilada por clave
- C2 Conversión engine/exhaust/bed/scrape a buses L/R
- C3 One-shots con polifonía (pool 4 voces/clave)
- C4 Envíos y retornos de reverb
- C5 Auditoría gain staging/headroom
- E1-E4 Tests unitarios, mixer sintético, banco real

### ⏸ Sprint 3: Sound Design en Marcha (Épico D)
- D1 Hot reload fuera del render
- D2 Transición click-free
- D3 Diagnóstico visible
- D4 CLI preflight

### ⏸ Sprint 4: Calidad y Entrega (Épicos E+F)
- E5-E6 Godot headless, escucha humana
- F1-F5 Migración JSON canónica, builds, commits, retrospectiva

---

## Archivos Modificados/Creados en Sprint 0

| Archivo | Acción | Descripción |
|---------|--------|-------------|
| `src/config.rs` | Modificado | Schema v2 tipos, validación, herencia (~900 líneas) |
| `src/mixer.rs` | Modificado | Usa `gains_map()` para compatibilidad |
| `src/bin/va_baseline.rs` | Creado | Herramienta de baseline determinista |
| `src/bin/va_validate.rs` | Creado | CLI validador/migrador |
| `tests/baseline_golden.json` | Creado | Métricas golden de 6 escenarios |
| `SOUND_CONFIG_DOCS.md` | Creado | Documentación completa schema v2 |
| `IMPLEMENTATION_PLAN.md` | Creado | Este documento |

## Definición de Done Global (ver instrucciones.txt §5)
- [x] JSON controla volume, pan, ADSR, EQ, tube, reverb por sonido (schema v2 definido)
- [ ] Engine/exhaust/superficies/scrape/shots usan cadena sin bypass accidental
- [ ] Estéreo real, ADSR sample-accurate, EQ medido, color sutil
- [ ] Buses reverb compartidos con RT60/color/width configurados
- [ ] Hot reload sin clicks/zipper/bloqueos
- [ ] Cero allocations/locks/I/O en render
- [ ] Sin NaN/Inf/DC/hard clip/samples sobre techo
- [ ] V1 compatible; tests pasan
- [ ] Build limpio = HEAD; commits atómicos
- [ ] Review + human gate + retrospectiva

## Procedencia
- Rama: f1-94
- HEAD: 87a8a85c7582aee3092c2a56f6c02a020e1bd7ed
- Status: cambios pendientes de commit (Sprint 0)
