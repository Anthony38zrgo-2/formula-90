# Línea base de rendimiento — integración Godot ↔ Rust (antes de la fachada)

> Fase F0 del plan de la fachada-orquestador (`formula90_core`). Documenta el estado
> previo medible derivado del código y el método para obtener la medición empírica
> con el profiler de Godot. Se re-medirá contra el mismo método tras F2.

## Contexto medible del runtime (antes de tocar nada)

| Magnitud | Valor | Fuente |
|---|---|---|
| Frecuencia de física | 120 Hz | `game/project.godot:37 common/physics_ticks_per_second=120` |
| Mix de audio | 44,1 kHz, búfer de 60 ms (≈2646 muestras) | `vehicle_audio_controller_native.cpp` `create_audio_nodes` |
| Latencia de salida Godot | 10 ms | `game/project.godot:45 audio/output_latency/buffer_size_ms=10` |
| Push de audio | 1 llamada Variant `push_frame` **por muestra** | `vehicle_audio_controller_native.cpp:444-456` |
| Lecturas de telemetría por frame físico | 6+ `vehicle->get(...)` + `detect_surface()` + `aggregate_slip()` | `vehicle_audio_controller_native.cpp:405-457` |
| Carga de módulos Rust | 3 DLL independientes + 3 `LoadLibrary`/`GetProcAddress`/checks ABI | `F90SimBridge`, `F194RustVehicle`, `VehicleAudioControllerNative` |
| Print de telemetría | `UtilityFunctions::print` cada 0,5 s (2/s) en hilo principal | `f90_sim_bridge.cpp:219-227` |

## Cuellos de botella conocidos (hipótesis del plan, por falsificar/confirmar)

1. **`push_frame` por muestra (dominante)** — a 44,1 kHz son ~44.000 dispatchs de
   método Variant por segundo (*cada uno* boxeando un `Vector2`), en el hilo
   principal dentro de `_physics_process`. Frente al render Rust (µs), el puente
   domina el coste del frame.
2. **Telemetría duplicada a través del puente** — física Rust → C++ → propiedades
   del nodo (`apply_core_telemetry`) → el controlador de audio vuelve a leerlas con
   `get()` y las re-empuja a Rust (`vehicle_audio_set_state`). Dos cruces de frontera
   para el mismo dato, además de `detect_surface()`/`aggregate_slip()` que vuelven a
   consultar colisiones que la física ya muestreó (12 raycasts en `_integrate_forces`).
3. **Desync audio↔física** — búfer de 60 ms vs. física a 120 Hz ⇒ hasta ~60 ms de
   retraso entre el estado físico y lo audible; los triggers (cambio de marcha)
   arrancan en frontera de frame, no de muestra. Además el mixer Rust ya dispara los
   one-shots de marcha internamente (`mixer.rs set_state`), y el C++ lo repite
   (`vehicle_audio_controller_native.cpp:434-441`) ⇒ **riesgo de doble trigger**.
4. **Resampling implícito** — generador a 44,1 kHz custom frente a la salida de
   Godot (10 ms, probablemente 48 kHz) ⇒ coste extra por pull.
5. **Eco GDScript** — `vehicle_audio_controller.gd` adjunto en escenas (misma lógica
   en GDScript) y `audio_telemetry.gd` con `find_children("*")` global al arranque y
   lectura de `last_weights`/`pitches` por frame (cada getter C++ asigna un
   `PackedFloat32Array` nuevo).

## Método de medición (repetible tras F2)

1. Ejecutar la sesión de referencia (`scripts/run_f1_94.ps1`, escena
   `f1_94_rust.tscn`) con **el mismo trazado y entrada** (o una sesión headless con
   `game_cli`/`core_cli` para números deterministas).
2. Con el profiler de Godot (Debug → Visual Profiler), capturar el tiempo de
   `_physics_process` de `VehicleAudioControllerNative` y el frame-time total durante
   10 s; guardar `reports/facade_baseline_<fecha>.csv`.
3. Contabilizar por instrumentación: ancho del bucle `push_frame` (nº de muestras por
   frame) y nº de `get()` de telemetría por frame.
4. Repetir el mismo método en F2 con el nodo `F90Core` y comparar.

## Estado final (registrado el 2026-XX-YY)

- [ ] Línea base empírica capturada (pendiente de sesión en profiler).
- [ ] **Nota (verificado por E2E P/Invoke + core_cli):** el orquestador Rust ya
  produce física + audio en un solo `f90_core_step` con un solo cruce de frontera por
  tick (ver `game/core` y `reports/facade_snapshot.bin`).
