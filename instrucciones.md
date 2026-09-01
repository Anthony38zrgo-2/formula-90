# Handoff autoritativo — V10 Rust GF480

Fecha: 2026-08-31

Workspace: `D:\Formula90s`

Rama: `fix/v10-audio-physical-boundary`

Script canónico: `run_f1_94.ps1`

## Objetivo inmediato

Continuar afinando exclusivamente el sintetizador V10 greenfield en Rust contra `98_int_max_5.wav`, sin perder lo alcanzado a 7499 RPM ni volver a concentrar toda la energía en la orden 5. No integrar todavía C++, Faust, Godot o `VehicleAudioEngine`; el renderer offline es el gate.

## Seguridad y procedencia

Antes de modificar:

```powershell
git branch --show-current
git rev-parse HEAD
git status --short
```

El worktree tiene numerosos cambios y artefactos ajenos. No limpiar, resetear, cambiar de rama ni incluirlos. Stagear rutas explícitas; `git add -A` está prohibido. No versionar WAV, stems, DLL, `target` ni reportes salvo autorización.

Pipeline: `sprint planning → backlog item → implement → review → human gate → sprint retrospective → done`.

Aplicar fail-fast: primero render estacionario y métricas; no ocultar defectos con sweeps o smoke tests redundantes.

## Historial y arquitectura

Base GF471 consolidada:

```text
e0497ac1 feat(audio): rebalance V10 orders and add dynamic sweep render
```

GF471 introdujo matriz `stem × orden`, notch dinámico de orden 5 sólo por rutas, menor compresión/saturación, firma A/B, deriva lenta, residual disparado por eventos y sweep/lift-and-coast configurable.

GF480, contenido en el commit que acompaña este handoff, conserva identidad de cilindro hasta la escena:

```text
Crankshaft 720°
  → 10 Cylinder independientes
      ├─ pressure_derivative[10]
      ├─ blowdown[10]
      └─ Header[10] → cylinder_headers[10]
             ├─ CylinderMechanicalPath[10]
             │    ├─ delay geométrico individual
             │    ├─ 3 resonancias cortas individuales
             │    └─ polaridad/filtro por posición y banco
             ├─ Collector A compartido (0–4)
             ├─ Collector B compartido (5–9)
             └─ suma mecánica → estructura, soportes y cockpit compartidos
```

Las rutas no son copias: RMS aproximado -51.7 a -54.4 dBFS; correlación media entre pares `-0.071`, mínimo `-0.671`, máximo `0.705`.

Costo Release para renderizar 10 s: GF471 `1451 ms`; GF480 `2105 ms`; incremento aproximado 45 %. No optimizar sacrificando identidad por cilindro.

## Archivos en alcance

- `game/crates/v10-engine-synth/src/engine.rs`: arrays individuales de derivada, blowdown y header; test de sumas A/B.
- `game/crates/v10-engine-synth/src/scene.rs`: `CylinderMechanicalPath[10]`, estructuras compartidas y rutas GF471.
- `game/crates/v10-engine-synth/src/bin/v10_render.rs`: stems `cylinder_mechanical_0..9`, suma, steady y sweep.
- `scripts/audio/analyze_v10_references.py`: opción `--forced-rpm`.
- `scripts/audio/analyze_acoustic_scene_orders.py`: fase, coherencia y contribución firmada por stem/orden.

No asumir que otros archivos dirty pertenecen a la tarea.

## Referencias y RPM correctas

Directorio:

```text
D:\ASETS\Automobilista_Grand_Prix_Evolution_Whills_v25.4\Automobilista_Grand_Prix_Evolution_Whills_v25.4\Automobilista\GameData\Vehicles\Grand_Prix_Evo\1998\_Sounds\ALT
```

- `98_int_low.wav`: afinación declarada 7499.31 RPM.
- `98_int_med.wav`: 8313.25 RPM.
- `98_int_max_5.wav`: afinación declarada 15537.94 RPM.

Advertencia: el contenido periódico de `max_5` corresponde aproximadamente a 15327 RPM: 127.725 Hz es orden 0.5 y 1277.25 Hz orden 5. El estimador ciego devuelve erróneamente 7663.5 RPM al tomar la media orden como fundamental. Para bins de órdenes usar `--forced-rpm 15327`; conservar 15537.94 sólo como control de reproducción del mod.

## Gate preservado a 7499 RPM

| Métrica | 98 low | GF471 | GF480 |
|---|---:|---:|---:|
| RMS dBFS | -12.18 | -12.53 | -13.37 |
| Crest dB | 11.46 | 10.38 | 10.91 |
| Orden 5 | 19.6 % | 23.8 % | 22.4 % |
| Energía armónica | 42.0 % | 47.8 % | 47.0 % |
| 80–250 Hz | 31 % | 25 % | 26 % |
| 250–600 Hz | 24 % | 37 % | 38 % |
| 600–2500 Hz | 42 % | 29 % | 28 % |

No degradar estos gates sin A/B humano. Mantener orden 5 aproximadamente entre 20–30 % en baja/media RPM y conservar el peso metálico.

Evidencia: `reports/audio/rust-greenfield/gf480_7499rpm_ten_cylinder_paths.wav`, su carpeta `_stems` y `gf480_target_comparison/`.

## Diagnóstico alineado a 15327 RPM

GF480 limpio: throttle `0.72`, load `0.66`, 8 s útiles, warmup 1 s, 44100 Hz, seed `4035964944`, peak `0.841`, limiter `0 dB`.

| Métrica | 98 max_5 | GF480 |
|---|---:|---:|
| RMS dBFS | -5.04 | -14.21 |
| Crest dB | 4.99 | 12.71 |
| Centroide Hz | 761 | 726 |
| Rolloff85 Hz | 1277 | 669 |
| Energía armónica | 39.2 % | 24.2 % |
| Orden 5 | 11.0 % | 0.8 % |
| 80–250 Hz | 26.3 % | 16.7 % |
| 250–600 Hz | 25.2 % | 63.2 % |
| 600–2500 Hz | 44.3 % | 13.0 % |
| 2500–7000 Hz | 3.5 % | 5.5 % |
| Corr. ciclos 720° | 0.936 | 0.607 |
| Desv. ciclos | 0.024 | 0.173 |
| Modulación | 0.67 Hz | 10.43 Hz |

El centroide parecido es engañoso: GF480 tiene exceso estrecho en 250–600 Hz, hueco en 600–2500 Hz y cola aguda algo excesiva.

Órdenes relativas:

| Orden | 98 max_5 | GF480 | Diferencia |
|---:|---:|---:|---:|
| 0.5 | 0.0 dB | -7.5 | -7.5 |
| 1.0 | -4.4 | -1.0 | +3.4 |
| 1.5 | -9.6 | 0.0 | +9.6; dominante incorrecta |
| 2.0 | -3.2 | -23.8 | -20.6 |
| 2.5 | -2.0 | -11.2 | -9.2 |
| 3.0 | -11.6 | -18.9 | -7.3 |
| 3.5 | -11.3 | -21.4 | -10.1 |
| 4.5 | -9.8 | -20.4 | -10.6 |
| 5.0 | -3.5 | -14.6 | -11.1 |

## Cancelaciones confirmadas

Orden 2:

```text
gearbox_housing      +2.65
metallic_structure   -1.33
engine_air           -0.57
rear_exhaust         +0.18
```

Orden 5:

```text
cylinder_head_covers +3.62
engine_cover         -2.36
engine_air           -0.69
metallic_structure   +0.55
rear_exhaust         -0.36
```

Las órdenes ya existen y se cancelan. No crear otra fuente. Corregir propagación/fase. `gearbox_housing`, seguido por `airbox_plenum`, domina 250–600 Hz. No usar EQ master.

Evidencia: `gf480_15327rpm_vs_98_int_max.wav`, su carpeta `_stems`, `gf480_vs_98_int_max_15327_aligned/` y `gf480_15327_order_matrix/` bajo `reports/audio/rust-greenfield/`.

## Máxima carga y headroom

A 15537.94 RPM, throttle `0.92`, load `0.90`: peak de escena `1.156994`, RMS `0.249069`, limiter interno `-0.711 dB`. La escena excede PCM16 antes de igualar el objetivo. No subir `output_gain`.

`max_5` está fuertemente procesado (crest 4.99 dB). Meta inicial razonable para GF480 a máxima carga: crest 6–7 dB, progresivo desde 10–12 dB a carga media, sin hard clipping.

## Backlog inmediato fail-fast

### HR-1 — Fase de órdenes 2 y 5 — P0

- Medir fase a 7499 y 15327 RPM.
- Ajustar delays/polaridades sólo en metal, engine air/cover, rear exhaust y head covers cuando corresponda.
- Preferir propagación física o dependencia continua de RPM; no un delay fijo que sólo pase un punto.
- No aplicar boost de órdenes ni notch al master.

Gate: órdenes 2 y 5 a menos de 6 dB del objetivo a 15327; orden 5 a 7499 permanece 20–30 %; cero clipping y limiter menor de 0.5 dB a carga nominal.

### HR-2 — Redistribuir cuerpo — P0, después de HR-1

- Reducir captura 250–600 de `gearbox_housing` y revisar `airbox_plenum` a alta RPM.
- Aumentar radiación útil de culata, colectores y estructura entre 700–1800 Hz.
- Transición continua dependiente de RPM/carga; sin presets abruptos ni ruido continuo.

Gate inicial a 15327: 250–600 `<=40 %`, 600–2500 `>=30 %`, 2500–7000 `2–5 %`, rolloff85 `>=950 Hz`.

### HR-3 — Firma de órdenes — P0

- Reducir orden 1.5.
- Recuperar 0.5, 2, 2.5 y 3–4.5 desde las diez rutas existentes.
- Inspeccionar contribución por cilindro/banco antes de añadir resonadores.

### HR-4 — Dinámica máxima carga — P1, después del timbre

- Compresión/saturación de bus dependiente de carga/RPM.
- Ataque que conserve combustión y release sin bombeo mecánico.
- Headroom antes de `output_gain`; safety limiter no crea timbre.

Gate: crest 6–7 dB, peak `<0.98`, cero samples clipped, safety limiter `>-0.5 dB`.

### HR-5 — Estabilidad temporal — P1

- Identificar modulación a 10.43 Hz.
- Usar alineación fraccional de ciclos a alta RPM.
- Conservar identidad determinista por cilindro.

Gate: deriva 0.7–1.2 Hz y dispersión entre ciclos 0.02–0.07 con medición confiable.

## Comandos reproducibles

```powershell
cargo test --manifest-path game/crates/v10-engine-synth/Cargo.toml

cargo run --release --manifest-path game/crates/v10-engine-synth/Cargo.toml --bin v10_render -- `
  --rpm 15327 --seconds 8 --warmup 1 --throttle 0.72 --load 0.66 `
  --sample-rate 44100 --seed 4035964944 --acoustic-scene `
  --out reports/audio/rust-greenfield/gf480_15327rpm_vs_98_int_max.wav `
  --stems-dir reports/audio/rust-greenfield/gf480_15327rpm_vs_98_int_max_stems

python scripts/audio/analyze_v10_references.py `
  "D:\ASETS\Automobilista_Grand_Prix_Evolution_Whills_v25.4\Automobilista_Grand_Prix_Evolution_Whills_v25.4\Automobilista\GameData\Vehicles\Grand_Prix_Evo\1998\_Sounds\ALT\98_int_max_5.wav" `
  reports/audio/rust-greenfield/gf480_15327rpm_vs_98_int_max.wav `
  --forced-rpm 15327 `
  --output-dir reports/audio/rust-greenfield/gf480_vs_98_int_max_15327_aligned

python scripts/audio/analyze_acoustic_scene_orders.py `
  reports/audio/rust-greenfield/gf480_15327rpm_vs_98_int_max_stems `
  --rpm 15327 --output-gain 2.90 --start 1 --end 4 --max-order 10 `
  --output-dir reports/audio/rust-greenfield/gf480_15327_order_matrix
```

## Reglas de decisión

- No añadir capas antes de comprobar cancelación por fase.
- No resolver una ruta con EQ master.
- No convertir diez rutas nuevamente en dos bancos duplicados.
- No igualar RMS antes de corregir clipping y crest.
- `max_5` es una capa procesada y mezclada/pitch-shifted en el juego, no presión cruda.
- Validar cada cambio a 7499 y 15327 RPM.
- Sólo el usuario aprueba el human gate perceptual.

## Estado del handoff

- GF471 consolidado en `e0497ac1`.
- GF480 implementado; 12 tests Rust pasan.
- Diez rutas exportadas como stems.
- Orden 5 a 7499 RPM dentro del gate.
- Comparación alineada de alta RPM completada.
- HR-1 es el siguiente trabajo recomendado.
- Sin integración al runtime del juego.
