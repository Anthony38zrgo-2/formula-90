# AUD-04 telemetry delivery

Fecha: 2026-09-05
Worktree: `D:\Formula90s-aud-01-07`  
Rama: `codex/aud-01-07`
HEAD base: `e5c8a6d9825742745c553e56d78f78077c399039`

## Implementación

`formula90-core` construye `MechanicalAudioState` directamente desde
`PowertrainState` y `VehicleConfig`. La carga usa la magnitud de
`clutch_torque` y conserva retención con torque firmado negativo; la reducción
de TC se incluye una sola vez en la carga positiva. El adaptador calcula
derivadas EMA de 20 ms, transmite clutch/TC, y genera únicamente fases físicas
de corte más una recuperación acústica de 40 ms. No crea un blip de downshift ni
dispara un segundo evento de cambio.

`RuntimeTelemetry` extiende el contrato interno con clutch, TC y limiter. El C
ABI `VehicleAudioTelemetryV3` no cambia. En GF509, clutch y TC son controles
continuos interpolados por muestra; shift y limiter son estados discretos. El
sintetizador consume torque firmado para retención, fase/limiter para la energía
de combustión y TC para una textura acotada, sin volver a reducir la carga.
La capa de samples también recibe torque firmado y clutch; su peso off usa
`max(low_load, -torque * clutch)`, preservando retención con carga alta y
manteniendo clutch-open free-rev fuera de ese stem.

## Evidencia

Pruebas focalizadas:

```text
cargo test -p formula90_core --lib --no-fail-fast
11 passed, 0 failed

cargo test -p formula90_core aud04_integration --no-fail-fast
2 passed, 0 failed

cargo test -p v10-engine-synth --lib --no-fail-fast
76 passed, 1 ignored, 0 failed
```

El build release se ejecutó con:

```text
cargo build --release -p formula90_core --bin aud_path_bench
```

El candidato aislado está en
`reports/audio-v10/aud04/candidate-20260905.json` (SHA-256
`FE706F032778F9161CC4F4D0E560BE6AF538DFAEBB177C6D6728E2A223B1B451`). El
baseline previo `reports/audio-v10/resume-20260905/before.json` se conservó
intacto (SHA-256 `322652F5CFAD1D657E126ECA21F8BA9E3BDF332ADBDEE4B10278197B476C2ACA`).

El WAV candidato post-AUD04 está en
`reports/audio-v10/aud04/audio-candidate-20260905/`: `gf509_runtime.wav`
(SHA-256 `BC4996C983898D6C5276EECC8D33E2E9B80DE69BB938BB3B7C1FD34BAF327F71`),
`vehicle_audio_engine.wav` (SHA-256
`35A50D0E1C5DA17165E4D0A75CADD80345DCFC7AE1FCA04E4FFD99F6860B28E2`), junto
con `trace.csv` (SHA-256 `A29B723FE31EEAA2E3BEC5E458B166D92264E6142C09AA58F4AC2FE13429739C`)
y `metadata.json` (SHA-256
`C2231CAA67DFB9EBFCA4F72AE5D50A6C7FA062C335DD0C79E292BBF72C17D113`) de
assets, fuente y HEAD.

Resultados release del candidato, 44.1 kHz / bloques de 256 / 4 096 bloques:

| Ruta | CPU | P50 | P95 | Allocaciones |
|---|---:|---:|---:|---:|
| Gf509RuntimeProcedural | 10.45% | 598.0 µs | 727.4 µs | 0 / 0 B |
| Gf509Runtime | 16.03% | 946.6 µs | 1 096.6 µs | 0 / 0 B |
| VehicleAudioEngine | 17.35% | 982.4 µs | 1 151.1 µs | 7 / 62 B |
| VehicleAudioEngineRenderOnly | 17.22% | 978.6 µs | 1 138.3 µs | 0 / 0 B |
| CoreFacade | 17.28% | 989.6 µs | 1 155.1 µs | 45 056 / 7.84 MB |
| CoreFacadeNoAudio | 0.13% | 5.2 µs | 5.3 µs | 45 056 / 7.84 MB |

La suite completa de `vehicle_audio_engine` no puede cerrar sus 7 tests C++
porque `f90_audio_dsp.dll` no está presente (`DllNotFound`). Esto no se cuenta
como evidencia de regresión AUD-04.

## Pendiente de gate

Faltan la comparación release con la misma matriz física (RPM sostenida,
aceleración, coast, free-rev/clutch, TC, upshift, downshift y limiter), la
verificación final BUILD/HEAD con `run_f1_94.ps1`, y la escucha humana
nivelada contra el corpus R25. El ítem queda técnicamente parcial, no cerrado.
