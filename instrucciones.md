# Handoff — mundo físico acoplado (`PhysicalWorldPackage`)

Fecha: 2026-10-19. Rama: `main-clean`.

- **HEAD:** `adcc109f8db03d27f02950c828ac31d99c9c69c1`
- **Commit atómico creado:** items 17 y 28–35.
- **`game/BUILD_SOURCE`:** `ce0f153c70c6b888964bea428ded5c300c5fe5ab` — **desactualizado**. El launcher canónico rechaza BUILD≠HEAD hasta rebuild.
- **Árbol:** item 43, item 46 y la frecuencia mixta jugador 120 Hz / IA 60 Hz están **sin commitear**.
- **Aprobación humana:** ningún item tiene `human_approved: true`. Nada está aprobado ni activado por defecto.
- **Cambios ajenos preservados:** audio, DLLs, `game/BUILD_SOURCE`, `recorded_engine_profile.rs`, `.import`/`.uid` generados y demás trabajo no relacionado.
- **Runtime:** no se ha ejecutado rebuild, launcher canónico, Godot MCP, revisión visual, audio ni benchmark de juego. Solo hay evidencia Rust offline.

---

## 1. Qué está commiteado

El commit `adcc109f` contiene 25 archivos y 4.266 inserciones:

- Items **17 y 28–35**: cargas de articulación, manifolds persistentes, solver normal/tangencial, CCD, clasificación, multi-vehículo, objetos dinámicos y reposo estable.
- Nuevos módulos `coupled_vehicle_loads.rs`, `dynamic_world_objects.rs` y sus tests.
- Cambios en `coupled_vehicle_dynamics.rs`, `coupled_vehicle_world.rs`, `coupled_vehicle_world_interface.rs`, `generalized_vehicle_contacts.rs`, `physical_collision_world.rs`, `suspension_*`, `lib.rs`.
- Documentación: contrato de arquitectura, backlog, candidato, integración Godot e `instrucciones.md`.

Fueron excluidos deliberadamente: `game/BUILD_SOURCE`, DLLs de `game/addons/formula90s/bin/`, audio, `recorded_engine_profile.rs` y artefactos generados.

---

## 2. Qué está sin commitear

### Item 43 — matriz de impacto candidata

- `game/crates/vehicle-physics-engine/tests/coupled_impact_matrix_test.rs`
- **11/11 tests:** frontal, morro contra asfalto, oblicuo, curb parcial, bottoming, aterrizaje con topes, sidewall, rueda-rueda, montonera 3/4 coches y 300 km/h sin túnel.
- Verifica verticalidad, penetración acotada, estado finito, eventos, islas y energía de colisión pasiva.
- Limitaciones: geometría sintética; la mayoría usa restitución 0 y fricción 1e-6; los impactos severos usan un techo angular diagnóstico de 60 rad/s (producción 10); la montonera de 4 coches usa tolerancia 1e-5 m/s (producción 1e-6).
- Evidencia: `scratch/m2_item43_impact_matrix_20261019/`.

### Item 46 — coste de carrera y frecuencia mixta

- `game/crates/vehicle-physics-engine/examples/coupled_vehicle_race_cost.rs`
- `docs/coupled_vehicle_race_cost_benchmark.md`
- **6/6 tests mixtos** en `coupled_mixed_frequency_test.rs`.
- Política acordada:
  - jugador 120 Hz;
  - toda la IA 60 Hz, actualizada en lote cada segundo tick de 120 Hz;
  - la fase IA precede a la fase jugador en el tick de actualización;
  - IA retenida entre actualizaciones, con subpaso máximo 1/60 s;
  - contactos jugador-IA a 120 Hz tratando la IA como entidad fija;
  - contactos IA-IA e IA-objeto solo en la fase IA;
  - objetos dinámicos a 120 Hz;
  - inputs aplicados en límites de 120 Hz; los de IA en la siguiente fase IA.
- Interfaz: `RegisterVehicle` acepta `physical_frequency_hertz` opcional; el controlador Godot expone `vehicle_physical_frequencies`. Solo se aceptan 60 Hz para IA y la frecuencia del mundo para jugador; 90 Hz se rechaza.
- Contrato actualizado con `mixed_frequency_policy`.
- Evidencia: `scratch/m2_item46_race_cost_20261019/`.

### Archivos modificados sin commitear

```text
docs/coupled_vehicle_architecture_contract.json
docs/coupled_vehicle_backlog_execution_status.json
docs/coupled_vehicle_candidate_implementation.md
docs/coupled_vehicle_godot_integration.md
instrucciones.md
game/crates/vehicle-physics-engine/src/coupled_vehicle_world.rs
game/crates/vehicle-physics-engine/src/generalized_vehicle_contacts.rs
game/scripts/runtime/physical_vehicle_world_controller.gd
```

### Archivos nuevos sin commitear

```text
docs/coupled_vehicle_race_cost_benchmark.md
game/crates/vehicle-physics-engine/examples/coupled_vehicle_race_cost.rs
game/crates/vehicle-physics-engine/tests/coupled_impact_matrix_test.rs
game/crates/vehicle-physics-engine/tests/coupled_mixed_frequency_test.rs
```

---

## 3. Verificación actual

Suites acopladas: **82/82**.

| Suite | Resultado |
|---|---|
| `coupled_vehicle_world_test` | 28/28 |
| `coupled_contact_manifold_test` | 10/10 |
| `coupled_contact_solver_test` | 6/6 |
| `coupled_continuous_collision_test` | 2/2 |
| `coupled_multi_vehicle_test` | 2/2 |
| `coupled_dynamic_object_test` | 7/7 |
| `coupled_rest_stability_test` | 5/5 |
| `coupled_impact_matrix_test` | 11/11 |
| `coupled_mixed_frequency_test` | 6/6 |
| `coupled_vehicle_load_report_test` | 5/5 |

`cargo check -p vehicle_physics_engine --lib`: limpio.

Suite completa: siguen exactamente los **14 fallos preexistentes** (1 lib aero, 4 `aero_test`, 1 `fuel_model_test`, 1 `regression_suite_test`, 7 `suspension_multibody_test`). No hay fallo nuevo.

Logs principales:

- `scratch/m2_item46_race_cost_20261019/cargo_test_full_suite_mixed_final.log`
- `scratch/m2_item46_race_cost_20261019/cargo_test_coupled_mixed_suite.log`
- `scratch/m2_item43_impact_matrix_20261019/cargo_test_full_suite.log`

---

## 4. Benchmarks y gate M3

Hardware: AMD64 Family 25 Model 80, 12 procesadores lógicos, Windows x86_64. Perfil geométrico, paquete físico Fuji y 50 plazas autoradas.

### Mundo uniforme (referencia)

| Escenario | Media | p95 |
|---|---:|---:|
| 1 coche idle | 1,88 ms | 3,32 ms |
| 20 coches idle | 39,0 ms | 62,9 ms |
| 20 coches rolling | 48,9 ms | 70,4 ms |
| 50 coches idle | 98,8 ms | 158,6 ms |
| 50 coches rolling | 129,1 ms | 189,1 ms |

### Frecuencia mixta (1 jugador + resto IA)

| Escenario | Frame jugador media | Frame jugador p95 | Fase IA media | Fase IA p95 |
|---|---:|---:|---:|---:|
| 20 mixto | 36,5 ms | 73,9 ms | 38,8 ms | 53,2 ms |
| 20 mixto rolling | 38,8 ms | 74,4 ms | 41,3 ms | 54,3 ms |
| 50 mixto | 95,5 ms | 184,7 ms | 103,2 ms | 137,3 ms |

### Gate propuesto (pendiente de aprobación)

- Frame jugador 120 Hz: media ≤30 % del intervalo (8,33 ms), p95 ≤50 % para 20 coches; 50 coches p95 ≤60 %.
- Lote IA 60 Hz: media ≤30 % de 16,67 ms, p95 ≤50 % para 20 coches; 50 coches p95 ≤60 %.

**El gate falla en todos los escenarios.** La media mixta mejora poco y el p95 empeora porque toda la IA se actualiza junta. El coste dominante es la evaluación por vehículo (~1,15–1,18 ms/coche/paso); el kernel aislado registra ~0,455 ms por evaluación completa y el mundo evalúa más de una vez por coche y paso. Geometría y matriz de contacto no son el cuello en estas parrillas.

Opciones que requieren decisión humana:

1. simplificar el modelo de producción para el mundo;
2. paralelizar evaluaciones por vehículo de forma determinista;
3. reducir evaluaciones por paso y coste del clon transaccional;
4. repartir la IA en dos grupos alternos (mejoraría p95, pero contradice “toda la IA junta”);
5. revisar el presupuesto M3.

---

## 5. Estado del backlog

- **18 `implemented_awaiting_review`:** 01–06, 09, 17, 28–37.
- **29 `implementing`:** 07, 08, 10–16, 18–27, 38–46, 50.
- **3 `pending`:** 47, 48, 49.
- `human_approved: false` en todos.

### Pendientes críticos por hito

- **M1:** 07, 08, 11–16, 18–20. Gate de derivadas, envolventes, energía/momento y cargas sin cerrar.
- **M2:** 24–27 (librería, geometría del mundo, formas, consultas); 43 parcial.
- **M3:** 22 y 46. El gate mixto falla.
- **M4:** 21, 38, 41, 42; 36/37 en revisión. La frecuencia mixta ya tiene interfaz y scheduler candidatos.
- **M5:** 39 y 40.
- **M6:** 44 y 45.
- **M7:** 43 matriz candidata, 47–50. **48/49/50 no pueden cerrarse sin rebuild y runtime canónico.**

Dependencias que siguen bloqueando el cierre:
- Item 16 (topes como eventos), 23 (bancos pre-colisión) y 42 (activación) para la matriz 43.
- Item 25 (promoción del paquete físico) para el runtime completo.
- Rebuild/paridad para cualquier aceptación runtime.

---

## 6. Próximos pasos recomendados

1. **Revisión humana y commit atómico de 43/46/mixta.** Stage explícito de los archivos listados en §2; nunca `git add -A`. Excluir DLLs, `BUILD_SOURCE`, audio y ajenos.
2. **Decidir el gate M3:** modelo, paralelismo, evaluaciones, agrupación de IA o presupuesto. Sin esa decisión, no tiene sentido seguir microoptimizando geometría.
3. **Cerrar dependencias de M2/M4:** 24–27 (geometría/formas/consultas), 16/23/42 (matriz de impacto y activación), 38/41 (ciclo de vida y consumidores).
4. **Rebuild y reseal:** tras el commit, ejecutar build completo y actualizar localmente `game/BUILD_SOURCE` como artefacto de build; no commitearlo.
5. **Validación Godot/runtime:** usar el servidor MCP de Godot (`godot_project_info`, escenas, logs, capturas) para la escena canónica. Los tests Rust no sustituyen la revisión visual, el audio ni el rendimiento real.

---

## 7. Reglas de continuación

- Verificar y registrar rama, HEAD, `git status` y `BUILD_SOURCE` antes de tocar nada.
- No cambiar de rama, resetear ni alterar un árbol sucio sin inventariar y respaldar.
- Preservar los cambios ajenos de audio, DLLs, `recorded_engine_profile.rs` y artefactos generados.
- Commits atómicos, staging por rutas explícitas, `git diff --cached` revisado.
- No reutilizar DLLs, objetos ni cachés entre ramas.
- No declarar gates superados sin evidencia fechada, SHA, hardware y aprobación humana.
- Mantener el destino `-PhysicalWorldPackage` como opt-in hasta aprobación explícita.
- Ninguna nota autoriza saltarse gates fallidos ni marcar los 50 items como completos.

---

## 8. Comandos de rebuild y runtime

```powershell
git rev-parse HEAD
Get-Content game\BUILD_SOURCE
scripts\build_windows.ps1 -Configuration debug
scripts\run_f1_97.ps1 -ValidateRuntimeOnly
scripts\run_f1_94.ps1 -PhysicalWorldPackage "<ruta absoluta del paquete>"
git status --porcelain
```

No commitear `game/BUILD_SOURCE`, DLLs, `game/.godot` ni artefactos de build. Cualquier commit mueve HEAD: rebuild obligatorio antes de lanzar.

---

## 9. Evidencia

- Commit atómico y alcance: `scratch/atomic_commit_items_17_28_35_20261019/`.
- Item 34: `scratch/m2_item34_dynamic_world_objects_20261019/`.
- Item 35: `scratch/m2_item35_rest_stability_20261019/`.
- Item 43: `scratch/m2_item43_impact_matrix_20261019/`.
- Item 46 y frecuencia mixta: `scratch/m2_item46_race_cost_20261019/`.
- Benchmark: `docs/coupled_vehicle_race_cost_benchmark.md`.
- Backlog autoritativo: `docs/coupled_vehicle_backlog_execution_status.json`.
