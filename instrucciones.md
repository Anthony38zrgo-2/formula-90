# Instrucciones — adopción definitiva del mundo físico acoplado (`PhysicalWorldPackage`)

Fecha: 2026-10-19. Rama: `main-clean`.
HEAD al escribir este documento: `ce0f153c70c6b888964bea428ded5c300c5fe5ab`.
`game/BUILD_SOURCE` al escribir: `ce0f153c70c6b888964bea428ded5c300c5fe5ab`.

**Decisión del usuario (2026-10-19):** la solución definitiva para el problema de colisiones y de autoridad física es el **mundo físico acoplado** (`scripts/run_f1_94.ps1 -PhysicalWorldPackage`), promovido a propietario por defecto **solo después de cerrar sus gates**. El modo actual (`RigidBody3D` de Godot + backends simplificados) se conserva como referencia A/B y para diagnóstico; no es el destino.

Este archivo es el **backlog autoritativo** del candidato (así lo declaran `docs/coupled_vehicle_backlog_execution_status.json` y `docs/coupled_vehicle_foundation_status.json`). Al avanzar items, actualizar el estado en `docs/coupled_vehicle_backlog_execution_status.json` (hoy: 9 `implemented_awaiting_review`, 34 `implementing`, 7 `pending`, 0 `complete`, `runtime_activated: false`, `human_approved: false`; su `source_head` está desactualizado).

---

## 0. Conocimiento para decidir (leer antes de tocar nada)

### 0.1 Por qué el mundo acoplado (el problema que resuelve)

El modo actual tiene **tres autoridades de contacto** sobre el mismo coche, sin arbitraje:

1. Godot: las 3 cajas (`CollisionTub/Nose/Rear`, fondos locales −0.18/−0.22) contra el mundo, con solver **duro**: corrige penetración + restringe velocidad, sobre el cuerpo entero, **sin tope de impulso**.
2. Rust: 12 raycasts + modelo de suspensión/neumático (Fz por compresión).
3. Rust: modelo `underfloor_contact` (activación 12 mm, hasta 12 kN por sonda) y su rama "rigid" que consume los contactos de Godot (`raw_contacts`).

Las cajas están ~0.30 m por encima del plano de las ruedas (fondo real medido: local −0.513751; reporte `coordinate_and_pivot_report.json`, `floor_height_from_ground_m = 0.024899`). Cualquier toque con el asfalto es penetración profunda → impulso grande → el coche salta → los rayos dejan de ver suelo → al caer, pico de compresión → rebote → vuelco. Rust no está roto: reacciona a un estado impuesto por un contacto rígido externo.

El mundo acoplado elimina la clase entera: los proxies nativos quedan **congelados** (capa/máscara 0, gravedad 0, integrador custom) y **Rust es dueño único de la integración y de los contactos** (respuesta generalizada con Parry: impulsos acotados, fricción/restitutión configurables, sin corrección de posición del motor). Es el enfoque de simulador (los contactos son del modelo, no del motor gráfico).

### 0.2 Contrato e invariantes que no se pueden romper

- **Un solo dueño de integración.** Bajo propiedad física, `F194RustVehicle::_integrate_forces` y el callback de fuerza legado retornan sin integrar; nunca dos avances del mismo vehículo.
- **Nunca avanzar una segunda simulación para presentación.** Los snapshots publican el estado resuelto; los renderers derivan solo transformadas rígidas. No hay un segundo solver de suspensión en GDScript.
- **Proxies inertes:** capa/máscara 0, `gravity_scale = 0`, `use_custom_integrator`, `freeze`. Un candidato fallido detiene el scheduling y congela sus cuerpos.
- **Paridad de fuente:** la librería del mundo exporta el commit fuente completo y su loader exige acuerdo con el identificador nativo y `BUILD_SOURCE`; librerías viejas se rechazan. Rebuilds nativo+Rust coordinados; cambios de layout ABI exigen revisión explícita (hoy `EXPECTED_ABI_VERSION = 21`).
- **Documento de perfil intacto:** el JSON del perfil se transporta sin cambios; la conversión numérica de Godot se normaliza solo para enteros; la serialización nativa usa precisión completa. No relajar validaciones de Rust.
- **Servicios, no mutaciones:** reset/refuel/recambio de neumáticos pasan por peticiones al mundo (republish inmediato de snapshots). Con propiedad física activa se rechaza el tuning directo del shadow.
- **Identidad y tiempo:** peticiones validadas estrictamente (identidad, tiempo físico, masa operativa). Snapshots de servicio a tiempo sin cambio no avanzan otro reloj.

### 0.3 Estado verificado (no volver a investigar esto)

- Integración Godot: `docs/coupled_vehicle_godot_integration.md`; activación explícita con `-PhysicalWorldPackage <ruta absoluta>`. **El launcher exige backends `standard`** con el paquete (simplified es solo para la integración Godot) y es incompatible con smoke/diagnóstico/livery.
- Paquete diagnóstico: `scratch/coupled_vehicle_complete_implementation/candidate_review_without_debug_symbols/fuji_physical_world.json` (74 mallas, 13.696 triángulos, transformaciones y códigos de superficie; SHA-256 de fuente + digest de geometría verificados por Rust). Es un artefacto de diagnóstico: se regenera con el flujo de extracción cuando cambie la geometría.
- Pruebas existentes: 25 tests Rust del mundo acoplado (`coupled_vehicle_world_test.rs`), suite geométrica (8), histeresis de neumático acoplado, y en Godot `test_physical_vehicle_world_interface.gd`, `test_physical_world_controller.gd`, `test_physical_world_native_presentation.gd` (2 pares vehículo/core reales, renderers reales, cero segundos solves de linkages).
- **Fallo de rendimiento abierto (bloqueante de activación):** 1 coche 0.5865 s por 0.2 s físicos; 32 coches 1.8755 s por 0.02 s; con caché geométrica compartida, 32 coches 1.3179 s por 20 ms ≈ **66× el intervalo pedido**. Kernel aislado: 0.271 ms trayectorias / 0.455 ms evaluación acoplada. No hay benchmark de percentiles ni de parrilla completa (Fuji tiene 50 posiciones de salida).
- **Fallos preexistentes que NO hay que perseguir** (reproducidos en HEAD limpio; ver `scratch/williams_aero_underfloor_sprint_20261018/logs/cargo_test_baseline_head.log`): `aero_test` (4), `fuel_model_test` (1), `regression_suite_test` (1), `suspension_multibody_test` (7) y el de librería `aero::tests::f1_2030_wheel_drag_reference_matches_profile_geometry`. El candidato documenta los mismos 4 de aero como ajenos.
- **Cambios ajenos que hay que preservar:** `game/crates/vehicle-audio-engine/src/grand_prix_sampler.rs` y `game/crates/formula90-core/tests/recorded_engine_profile.rs` (modificados por el usuario, sin commitear). Assets no rastreados: no tocar ni commitear en bloque.
- **Sprint aero/fondo (sin commitear al escribir esto):** corrección del reparto de `NodePath` nativo, referencia de sondas de fondo declarada por perfil, balance aero por estaciones reales de eje, recalibración del Williams (ClA 2.070 / 47% delantero) y goldens. Evidencia en `scratch/williams_aero_underfloor_sprint_20261018/` (incluye `review_status.md`). Requiere aprobación humana para commit; el rebuild posterior sella BUILD/HEAD. Es conocimiento necesario para el mundo acoplado: la misma referencia de fondo y el mismo criterio de estaciones deben respetarse en su modelo aero.
- **Decisión de arquitectura pendiente de relacionar:** existe una propuesta de backend `lite` de bajo coste (`formula90_main_clean_v2_fisicas_ligeras.md`). El mundo acoplado y `lite` son alternativas de coste/precisión: si el mundo acoplado se adopta como definitivo, confirmar si `lite` se cancela o se mantiene como segundo nivel.

### 0.4 Mapa de archivos (dónde vive cada cosa)

| Dominio | Rutas |
|---|---|
| Contratos nativos | `native/include/formula90s/core/physical_world_document.hpp`, `physical_vehicle_world_interface.hpp`, `f90_core.h`, `f90_core.hpp` |
| Implementación nativa | `native/src/core/physical_vehicle_world_interface.cpp`, `f90_core.cpp` (aceptación de snapshots), `native/src/vehicle/f1_94_rust_vehicle.cpp` (rutas `physical_world_controlled`) |
| Controlador Godot | `game/scripts/runtime/physical_vehicle_world_controller.gd` |
| Mundo Rust | `game/crates/vehicle-physics-engine/src/coupled_vehicle_world.rs`, `coupled_vehicle_world_interface.rs`, `coupled_vehicle_dynamics.rs`, `generalized_vehicle_contacts.rs`, `physical_collision_world.rs`, `vehicle_mass_inventory.rs`, `suspension_*`, `static_lateral_contact.rs` |
| Fachada | `game/crates/formula90-core/src/lib.rs`, `ffi.rs`, `frame.rs`, `underfloor.rs` |
| Contratos y estado | `docs/coupled_vehicle_architecture_contract.json`, `coupled_vehicle_validation_specification.json`, `coupled_vehicle_foundation_{definition,scenario,status}.json`, `coupled_vehicle_foundation_review.md`, `coupled_vehicle_candidate_implementation.md`, `coupled_vehicle_backlog_execution_status.json` |
| Evidencia | `scratch/coupled_vehicle_godot_integration/`, `scratch/coupled_vehicle_complete_implementation/`, `scratch/coupled_vehicle_foundation_20261005_final/`, `scratch/coupled_vehicle_godot_runtime_rebuild/` |
| Pista | `game/tracks/fuji76_77/` (`fuji76_77_collision.glb`, `metadata/package.json` con hashes validados por el launcher) y `game/scripts/track/imported_track_collision.gd` (grupos de superficie) |

### 0.5 Reglas de medida (de la especificación de validación)

- Error relativo = error absoluto / max(|referencia|, piso explícito del escenario).
- Residuo de energía = energía final + disipada − cambio por colisión − trabajo externo − trabajo del actuador de dirección − inicial (con ledgers acumulados).
- Intercambio de momento incluye combustible y el impulso externo; el origen del momento angular permanece fijo.
- **Convergencia suave y de impacto se miden por separado.**
- Incertidumbre declarada por componente: masa ±20%, inercia ±30%, centro ±0.05 m.
- Ningún gate se declara superado sin reporte; los presupuestos de rendimiento y calibración de release siguen pendientes de aprobación.

---

## 1. Backlog por hitos

Cada hito: objetivo, items (numeración del backlog de 50 en `docs/coupled_vehicle_backlog_execution_status.json`), entregable, gate y evidencia requerida.

### M1 — Cerrar el modelo físico del vehículo (items 7, 8, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20)

Objetivo: modelo acoplado completo y diferenciable (masa/inercia, envolventes geométricas, matriz generalizada, cargas de articulación, trabajo de neumático/tren motriz).

- 7-8: rangos de estimación con evidencia por componente; centros y tensores completos sobre el dominio operativo (barridos de dirección y recorrido).
- 11-12: envolventes finitas contra cárter, caja, transmisiones y miembros adyacentes; derivadas espaciales con diferencias finitas centradas a varios tamaños de perturbación (traslación y rotación separadas).
- 13-14: matriz de masa de producción, comparación contra referencia, autovalores generalizados y condicionamiento; inercia de rotores girando y energía de dirección prescrita.
- 15-16: proyección de fuerzas de suspensión por trabajo mecánico; topes de recorrido como eventos mecánicos unificados con freno y colisión (cruces simultáneos).
- 17: cargas físicas de articulación y anclaje recuperadas de aceleraciones y reacciones de restricción.
- 18: momento y energía rotacional completos de motor, ejes y embrague; restricciones simultáneas.
- 19: modelo de neumático conectado al movimiento real de contacto (huella, rigidez vertical dependiente de presión, diagnóstico de deflexión, trabajo de deslizamiento).
- 20: proyección de fuerzas aerodinámicas y externas restantes con reconstrucción real del suelo y trabajo de cada fuerza. **Usar la referencia de fondo declarada y las estaciones reales de eje del sprint aero/fondo; no reintroducir alturas absolutas.**

Gate: gates de fundación (9, ver §2) verdes + los gates de derivadas del hito con reporte (traslación/rotación separadas, varias escalas) + cierre de energía/momento dentro del presupuesto aprobado.

### M2 — Contacto y mundo físico (items 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35)

Objetivo: **esta es la parte que resuelve el problema reportado** (impulsos acotados y con una sola autoridad). Contacto persistente, materiales, CCD y clasificación sin doble respuesta.

- 24: selección de librería geométrica revisada (licencia, features, mantenimiento).
- 25-26: geometría del mundo importada con cobertura y grupos de superficie validados; formas del vehículo articulado con fidelidad y límites finitos.
- 27: broad phase escalable y filtrado de candidatos para todos los dominios de consulta.
- 28/35: **manifolds persistentes** (por características), ciclo de vida de caché, warm start, fricción estática y estabilidad en reposo (incluye costuras de malla).
- 29: impulsos normales resueltos con la matriz generalizada del vehículo (contactos simultáneos persistentes, recorrido y freno).
- 30: fricción tangencial de impacto y efectos rotacionales; pares de materiales; matriz de pasividad completa.
- 31: CCD y localización de eventos; error de trayectoria articulada acotado; túnel y convergencia de eventos simultáneos.
- 32: clasificación de contactos y **prohibición de respuesta duplicada** (pianos compuestos, remapeo de características, pendientes).
- 33: contacto vehículo-vehículo en un solo paso de mundo (impacto múltiple, montonera, identidad/eliminación, energía).
- 34: interoperabilidad con objetos dinámicos del mundo (propiedad, intercambio de momento, ciclo de vida).

Gate (matriz de impacto y contacto, item 43): morro contra asfalto **no puede volcar ni saltar** (no hay contacto rígido del motor: el suelo es modelo de contacto); golpe lateral contra muro con impulso acotado y sin ganancia de energía; piano parcial; aterrizaje tras salto; reposo estable sin jitter; sin túnel a 300 km/h; pasividad en todos los pares de material. Evidencia: telemetría de impulsos por contacto, residuo de energía por escenario y matriz completa.

### M3 — Integrador y coste de carrera (items 22, 46)

Objetivo: método y frecuencia de producción seleccionados con convergencia, pasividad y **coste medido**.

- 22: método de integración y frecuencia elegidos con convergencia suave/impacto separada y pasividad.
- 46: medir y optimizar sin cambiar física: cuellos de botella de geometría/matriz; mediana y cola en todas las posiciones de salida de la parrilla.

Gate propuesto (requiere aprobación humana, hoy es una propuesta): a 120 Hz, paso físico por coche con presupuesto de parrilla real (1/4/12/20+ y las 50 posiciones de Fuji) — media ≤ 30% del intervalo (8.33 ms) para 20 coches y p95 ≤ 50%; 50 coches ≤ 60% de media; sin asignaciones en el paso; benchmark reproducible con BUILD/HEAD y hardware registrados. Estado actual: **66× el intervalo** (bloqueante).

### M4 — Runtime, scheduling y activación (items 21, 36, 37, 38, 41, 42)

Objetivo: un solo paso de mundo en la escena canónica, ciclo de vida completo y activación explícita sin cambiar el propietario por defecto hasta aprobación.

- 21/36/37: mundo autoritativo y interfaz versionada; el bucle de fuerza del host se reemplaza por **un paso de mundo** (prioridad 1000, fracción de host retenida, una sola vez por frame); escena canónica completa y scheduling de carrera.
- 38: ciclo de vida (reset, pausa, recomposición tardía de sesión, pit stop).
- 41: consumidores de gameplay y configuración (tuning en runtime, animación de boxes, cámaras, editores).
- 42: gates de activación y compatibilidad; mantener el propietario por defecto sin cambios hasta aprobación humana.

Gate: `-PhysicalWorldPackage` en la escena canónica con paridad de fuente; un solo avance por frame (test de orden de inputs y fracción retenida); pausa sin consumir tiempo físico; servicios (reset/refuel/recambio) verificados en runtime; sin errores de autoload/telemetría (hoy el harness nativo registra errores de `front_left_wheel`/`get_telemetry_snapshot`: corregir o documentar).

### M5 — Presentación, telemetría y audio (items 39, 40)

- 39: estado mecánico resuelto renderizado (hubs, bases, rocker, extremos de articulación, giro); interpolación y volúmenes de empaquetado con **revisión visual humana**.
- 40: telemetría y eventos de colisión a los consumidores existentes (audio de impacto con cooldown 0.2 s, raspado sostenido, feedback de volante, HUD, todas las clases de evento).

Gate: revisión visual humana del coche completo; audio de contacto audible y sin artefactos; HUD coherente con snapshots; cero segundos solves.

### M6 — Paridad y sensibilidad (items 44, 45)

- 44: paridad replay/standalone↔runtime (entradas grabadas, equivalencia entre pasos de host).
- 45: incertidumbre correlacionada (centros/tensores) y sensibilidad al paso temporal, contra la paridad integrada.

Gate: replay byte-idéntico a iguales entradas y paso; sensibilidad al paso dentro del presupuesto aprobado; incertidumbre reportada, no promediada.

### M7 — Calibración, build limpio y publicación (items 43, 47, 48, 49, 50)

- 43: matriz completa de contacto/impacto (ver M2).
- 47: calibración completa contra objetivos declarados (altura, neumático, manejo, cargas de reacción, respuesta de contacto).
- 48: build limpio con fuente que coincida (Cargo/SCons/Godot/DSP), huella de fuentes sucias registrada, launcher canónico verificado.
- 49: conducción integrada en Fuji + revisión visual + aceptación humana.
- 50: auditoría de entrega y publicación atómica revisada.

Gate final: todos los gates anteriores con evidencia fechada y SHA; `human_approved` en el estado del backlog; activación por defecto **solo** después de la aprobación humana explícita.

---

## 2. Gates de fundación (ya definidos, no redefinir)

De `docs/coupled_vehicle_validation_specification.json`:

- `existing_reference_tests`: casos nombrados de Cargo; 28 pases esperados en las 3 suites declaradas.
- Gates de fundación (9) y gates de desarrollo futuro (9) con `must_not_claim_pass_without_report`.
- Fixture: perfil `game/data/vehicles/f1_2030/f1_2030_v10_geometric.json` (o su sucesor versionado), definición `coupled_vehicle_foundation_definition.json`, escenario `coupled_vehicle_foundation_scenario.json`, clasificación `five_component_estimate_diagnostic_only`, sin aleatoriedad, geometría de hardpoints + superficies planas suministradas.
- La suite geométrica tiene una falla conocida y atribuida: no ocultarla ni "arreglarla" sin evidencia.

---

## 3. Checklist de rebuild, ejecución y entrega

```powershell
git rev-parse HEAD
Get-Content game\BUILD_SOURCE
scripts\build_windows.ps1 -Configuration debug
scripts\run_f1_97.ps1 -ValidateRuntimeOnly
scripts\run_f1_94.ps1 -PhysicalWorldPackage "<ruta absoluta del paquete>"
git status --porcelain
```

- No commitear `game/BUILD_SOURCE`, DLLs, `game/.godot` ni artefactos de build.
- Commits atómicos, staging por rutas explícitas; nunca `git add -A`.
- No cambiar de rama, no resetear, no reutilizar DLLs de otra rama.
- Cualquier commit mueve HEAD: **rebuild obligatorio** antes de lanzar (el launcher rechaza BUILD≠HEAD).
- Registrar BUILD/HEAD, hardware y perfil en cada evidencia.
- Preservar los cambios ajenos de audio y el test de `recorded_engine_profile.rs`.

## 4. Decisiones abiertas para el humano

1. Sprint aero/fondo (sin commitear): aprobar su commit; luego el rebuild sella BUILD/HEAD.
2. Relación con el backend `lite` de bajo coste: cancelar o mantener como segundo nivel.
3. Presupuesto de rendimiento de M3 (la propuesta del §1-M3 requiere aprobación).
4. Altura estática del Williams (documentada 30/45 mm vs +50 mm actual) y residual de drag (~6-12% bajo el CdA documentado).
5. Divergencia de barras entre backends (simplified 1.00/0.80 vs standard 0.15/0.09): unificar o publicar rigidez efectiva.

## 5. Límites

- El modo `standard` del motor de vehículo debe conservar ecuaciones, referencias, contactos y masa.
- No seleccionar correcciones por nombre de coche ni aplicar cambios globales al perfil compartido para corregir un solo coche.
- Ninguna nota de este documento autoriza saltarse gates fallidos ni marcar los 50 items como completos.
- La activación por defecto del mundo acoplado exige aprobación humana explícita; hasta entonces permanece opt-in.
