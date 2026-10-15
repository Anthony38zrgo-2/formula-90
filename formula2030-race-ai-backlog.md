# Formula 2030 — Backlog de IA de carrera y parrilla multivehículo

**Versión:** 1.1 (plan y seguimiento de implementación)<br>
**Fecha:** 2026-10-15<br>
**Destino:** repositorio [`Anthony38zrgo-2/formula-90`, rama `main-clean`](https://github.com/Anthony38zrgo-2/formula-90/tree/main-clean)<br>
**Estado de este documento:** implementación candidata de M1 disponible en el árbol de trabajo; **P0 y M1 no aprobados** por inestabilidad/contactos físicos y validación de conducción incompleta. Las casillas de aceptación permanecen abiertas; el avance de código se registra por separado a continuación.<br>
**Principio rector:** primero hacer que **un coche conduzca solo siguiendo la racing line**, luego una **parrilla de cuatro** estable; después añadir adelantamientos, personalidad y estrategia.

## 0. Resumen de decisiones (leer antes de programar)

1. **P0, prioridad absoluta: coste de simulación.** Probar primero el mundo físico Rust existente. El `CoupledVehicleWorld` realiza actualmente una consulta principal de suelo por rueda en su función de avance de fuerzas y duplica ese resultado en los tres campos de `TriRaycastSample`. **Eso no significa que el vehículo ejecute únicamente cuatro consultas de colisión totales**: existen consultas del piso aerodinámico, contactos de carrocería/neumáticos, solvers y otras operaciones. Medir antes de simplificar.
2. **Un solo dueño de la física.** Los pilotos jugador/IA comparten el mundo físico autoritativo Rust; se prohíben un segundo integrador de física, estados duplicados con velocidades inventadas o teletransporte de IA para resolver una maniobra.
3. **Perfil inicial para IA: `AI_LITE`**, si los benchmarks lo justifican. Objetivo: una muestra del suelo por rueda; conservar suspensión funcional, neumáticos con transferencia de carga, tracción, frenado, colisiones vehículo-vehículo y respuesta a bordillos. Reducir **detalle**, no crear ventajas competitivas falsas.
4. **Audio inicial por IA:** exclusivamente sonido de **motor**, con un máximo de una voz/stream lógico por vehículo AI audible, actualización de telemetría desacoplada, prioridad según distancia y límite de mezclas simultáneas. Sin caja, plank, neumáticos, ambiente por IA, ni eventos de choque sonoros inicialmente; las colisiones físicas siguen funcionando.
5. **M1 — mínimo jugable:** un coche IA en Fuji, sigue racing line, frena, acelera, cambia de marcha, completa 10 vueltas consecutivas con métricas registradas. Sin IA estratégica ni rivales.
6. **M2 — parrilla de 4:** cuatro coches compartiendo física, spawns correctos, ritmo distinto, sin adelantamientos al principio; detección/seguimiento básico para evitar embestidas. La carrera se mide y reproduce con semillas fijas.
7. **Después:** racecraft local → oportunidad de adelantamiento → conducción en paralelo → perfiles de piloto → objetivos del muro de boxes → estrategia adaptativa.
8. **Macro AI imparcial:** calcula objetivos estratégicos individuales, a baja frecuencia y por eventos, como un muro de boxes; **no maneja pedal/volante ni dirige artificialmente los adelantamientos**. La IA local puede retrasar/rechazar objetivos por seguridad y puede adelantar sin orden global.
9. **Orden de prioridades local:** self-preservation / límites físicos / reglas de carrera (restricciones no negociables), conducta deportiva, ejecución táctica/reactiva, ritmo, y por último utilidad de objetivos estratégicos. Un objetivo global modifica incentivos, nunca anula una restricción.
10. **Criterio de éxito:** estabilidad física, tiempo de simulación, consistencia y batallas plausibles; no "ganar por cualquier medio". **Prohibido rubber-banding** y cualquier multiplicador escondido de velocidad/grip para fabricar adelantamientos.

### 0.1 Estado real de implementación — corte 2026-10-15

**Procedencia:** rama `main-clean`, HEAD `f7fa7d46b559538a8dd80819f2f52e0dd7997573`, con cambios locales sin publicar. Los resultados corresponden a instantáneas diferentes del candidato; un resultado anterior no certifica el código posterior. Inventario previo a esta actualización: `scratch/race_driver_physical_pace_20261015/status_before_backlog_update.txt`.

| Área | Avance concreto | Estado de aceptación |
|---|---|---|
| M1-01 — trayectoria | Crate `game/crates/race-intelligence-engine`; lector de racing line, validación del circuito cerrado y cola histórica de Fuji, proyección longitudinal/lateral, continuidad y anchura útil del vehículo | Implementado en candidato; pendiente validación sostenida de la puerta M1 |
| M1-02 — velocidad | `track/vehicle_performance.rs`: envolvente nominal a partir de masa/combustible, neumáticos, sensibilidad a carga, carga aerodinámica, frenos y relaciones; límite por curvatura y pasada de frenado con reserva para giro y pendiente | Implementado en candidato; **no equivale a límite físico dinámico demostrado**. Falta validar aceleración factible, condiciones variables y diez vueltas |
| M1-03 — dirección/recuperación | Seguimiento geométrico con anticipación dependiente de velocidad y límite de variación; `steering_rack_calibration.rs` convierte ángulo de rueda a mando real de cremallera; recuperación por borde del lado correcto, huella y reducción continua de velocidad | Implementado parcialmente; no validado a alta velocidad. Falta amortiguar oscilaciones de seguimiento y comprobar perturbaciones sin mezclar defectos del controlador y de física |
| M1-04 — pedales | Objetivo de velocidad en la posición actual, eliminación de la doble anticipación de frenada, frenado anticipado por perfil, realimentación con anti-windup, banda de seguimiento, aceleración a fondo bajo objetivo y exclusión acelerador/freno | Implementado en candidato; pendiente suavidad y consistencia en pista con física estable |
| M1-04 — automático | Ajustes exclusivos de IA, régimen de subida, margen útil de la siguiente marcha, protección de reducciones y tiempo mínimo entre cambios. Eliminado bloqueo redundante de rpm de carretera que retenía segunda al corte con deslizamiento normal | Siete regresiones de selección aprobadas. Hubo cuarta marcha en banco y aproximadamente 298 km/h en Fuji; no certifica conducción sostenida ni calidad de todos los cambios |
| Cadencias | Física configurada a **120 Hz**; control del adaptador a **60 Hz**; scheduler conserva fracciones de tiempo y cuenta actualizaciones vencidas | Implementado y comprobado en escenarios individuales; falta determinismo y rendimiento de parrilla |
| Adaptación Godot | `src/autonomous_driver_bridge.rs`: transporte local de observaciones/mandos. `game/tests/test_autonomous_driver_fuji.gd`: un único mundo físico, paquete real de colisión Fuji, snapshots nativos y presentación. `scripts/run_autonomous_driver_fuji.ps1`: procedencia, compilación y ejecución | Adaptador de validación opt-in; **no integrado como carrera comercial ni selector humano/IA completo** |
| Telemetría | CSV de posición longitudinal, error lateral, objetivo/velocidad, dirección, pedales, marcha/rpm y recuperación; informe JSON, logs y diagnósticos físicos por snapshot | Disponible en pruebas; faltan todas las métricas y gráficas exigidas por P0/M1 |
| M2–M5 | Parrilla, percepción, following, adelantamientos, personalidad y estrategia | **No implementados ni autorizados por una puerta aprobada** |
| Audio IA / AI_LITE | Documentos de política y propuesta presentes | Sin implementación y validación suficientes para aprobar P0-02/P0-04; el adaptador no prueba cuatro voces de IA ni paridad física |

La configuración `game/data/race_intelligence/unrestricted_driver_tuning.json` utiliza `maximum_speed_metres_per_second: null`: **no hay límite global artificial de velocidad** en ese ensayo. Se mantienen restricciones de curva/frenada del perfil y los límites mecánicos del vehículo. `solitary_driver_tuning.json` conserva un límite para comparaciones históricas; no confundir ambos modos.

**Limitante actual, prioridad absoluta: estabilidad física/contactos del candidato a 120 Hz.** El ritmo de la IA no puede aceptarse mientras el vehículo desarrolle oscilaciones, pierda apoyo, alcance los topes o falle al resolver contactos. Tampoco está demostrado que todos los fallos restantes procedan exclusivamente de física: el controlador todavía requiere validación. No compensar estos problemas con agarre ficticio, límites de colisión relajados o cambios aleatorios de rebote.

#### Evidencia y límites de los resultados

| Evidencia local | Resultado medido | Interpretación |
|---|---|---|
| `scratch/autonomous_driver_fuji_120_hertz_verified_20261014/fuji_report.json` | Una vuelta renderizada de 4.310,55 m, 145,75 s simulados, máximo lateral 1,63957 m, cero salidas; 120 Hz físicos/60 Hz de control | **Éxito histórico con perfil limitado**, anterior a los cambios de ritmo/integrador; no cumple diez vueltas ni certifica el candidato actual |
| `scratch/autonomous_driver_fuji_physical_pace_20261015/fuji_report.json` | Ensayo sin límite global: fallo del dominio de velocidad de colisión a 15,9167 s, 585,42 m, cero salidas antes del fallo | Prueba fallida; automático retenía segunda cerca del corte antes de su corrección posterior |
| `scratch/autonomous_driver_fuji_limiter_corrected_20261015/fuji_report.json` | Tras corregir automático: máximo 296,83 km/h; fallo físico a 15,9 s, 627,54 m | El desbloqueo de marchas está observado; **ninguna vuelta completa** |
| `scratch/autonomous_driver_fuji_integrator_corrected_20261015/fuji_report.json` | Primera versión de amortiguación implícita: máximo 297,74 km/h; salida a 15,3 s y 607,92 m, máximo lateral 5,5535 m | **No aprobado**. Diagnóstico final: inclinación 40,35°. No atribuir este resultado a una IA ya calibrada o a un vuelco completamente resuelto |
| `scratch/race_driver_physical_pace_20261015/integrator_acceptance_tests.log` | Cinco regresiones aprobadas: amortiguador analítico pasivo, asentamiento en ambas rutas, excitación lateral repetida y topes simultáneos; residuo energético final del asentamiento −0,01443 J | Corrige una reproducción aislada; no sustituye los ensayos de pista ni certifica todo el integrador |
| `scratch/race_driver_physical_pace_20261015/integrator_localization_regression.log` | Última prueba cerrada sobre suelo plano falla con `Vehicle/world overlap exceeds the supported initialization/recovery domain`, después de superar los primeros 22 s trazados | **Regresión vigente y bloqueante**; la batería de IA no está completamente verde |
| `scratch/race_driver_physical_pace_20261015/integrator_replay_diagnosis.log` | Seis casos ejecutados correctamente; el replay entre cadencias no llega a comparar física: deserialización de `36.0` donde Rust esperaba `u32` | Arreglar el adaptador de diagnóstico. **No existe evidencia de convergencia de ese replay** |

#### Causa raíz confirmada y corrección física todavía candidata

El banco sin IA, sin acelerador y sin fuerzas laterales reprodujo oscilaciones persistentes. La amortiguación se calculaba con la velocidad inicial y se aplicaba explícitamente durante todo el paso, aunque la posición se actualizaba de forma semiimplícita. La estimación local de un modo rápido fue aproximadamente 261,48 por segundo: a 120 Hz, el factor explícito aproximado `1 − lambda × dt` era −1,179 y amplificaba la oscilación. La ley continua tenía signo disipativo; su discretización no garantizaba pasividad.

La implementación actual añade `src/suspension_damping.rs`, común a las rutas de suspensión de diez y catorce velocidades. Resuelve implícitamente en el punto medio los amortiguadores digresivos, la amortiguación vertical unilateral del neumático y la de los topes, con matriz de masas acoplada y trabajo disipado discreto. Conserva los coeficientes físicos y los **120 Hz**; no se ha ejecutado la propuesta de simplificar masas/componentes.

En tres segundos del banco, la disipación pasa de 7.398,42 J a unos 174,15 J; la velocidad residual de recorrido pasa de aproximadamente 0,495 m/s a 0,00000466 m/s. El error energético transitorio absoluto sigue siendo medible, aproximadamente 25,31 J en ese ensayo a 120 Hz: **residuo final pequeño no implica error transitorio cero**. Evidencia: `suspension_oscillation_audit.log`, `integrator_oscillation_final.log` e `integrator_replay_diagnosis.log`, dentro de `scratch/race_driver_physical_pace_20261015/`.

La localización de topes se ha adaptado posteriormente al paso implícito, reemplazando una iteración que no convergía por una búsqueda de intervalo admisible. La prueba de suelo plano posterior ya alcanza otro fallo de contactos. **No hay vuelta completa aprobada de la última revisión.** Una reconstrucción anterior del runtime verificó diez copias de bibliotecas y BUILD/HEAD, pero la última modificación del localizador aún requiere reconstrucción/despliegue y nueva verificación; no asumir que la DLL instalada contiene el último texto fuente.

Otros fallos registrados: la suite de biblioteca física da 159 aprobados y un fallo de referencia aerodinámica de ruedas; la suite heredada de multicuerpo da nueve aprobados y siete fallos de inventario de masa antes de integrar. Mantenerlos identificados por causa y revisar su procedencia; no presentarlos como fallos de IA ni alterar calibración para silenciarlos.

#### Orden de reanudación

1. Resolver **P0-06**: fallo de contactos/solapamiento y estabilidad del conjunto a 120 Hz, incluyendo interacciones entre contacto, topes, frenado y pérdida de apoyo. Mantener congelada la calibración de ritmo.
2. Resolver **P0-07**: normalización de enteros del diagnóstico Godot→Rust y replay de inputs idénticos entre cadencias; conservar 120 Hz como configuración de producción.
3. Reconstruir desde la última revisión, verificar bibliotecas/procedencia y repetir banco, tests de IA, smoke canónico y Fuji sin límite global.
4. Solo entonces retomar M1-02/M1-03/M1-04: suavidad de mandos y fidelidad del perfil nominal con la física corregida.
5. Ejecutar la puerta M1 completa: tres seeds × dos reinicios × diez vueltas y métricas exigidas. P0/M1 siguen abiertos y M2–M5 no avanzan.

Documentación relacionada: `docs/autonomous_driver_fuji_validation.md`, `docs/suspension_oscillation_audit_20261015.md` y `docs/suspension_simulation_architecture.md`. Estos documentos contienen resultados históricos y deben leerse junto a este corte de estado, que registra las limitaciones posteriores.

## 1. Auditoría inicial del repositorio (hechos observados; verificar otra vez sobre HEAD de trabajo)

| Recurso existente | Ubicación confirmada en `main-clean` | Implicación | Acción |
|---|---|---|---|
| Mundo físico autoritativo Rust multivehículo | [`game/crates/vehicle-physics-engine/src/coupled_vehicle_world.rs`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/vehicle-physics-engine/src/coupled_vehicle_world.rs) | Ya ofrece registro de vehículos, inputs, snapshots y colisiones | Reutilizar, perfilar, extender con cuidado |
| Consulta de suelo | `CoupledVehicleWorld::advance_vehicle_forces` en el archivo anterior | Una consulta principal por rueda, resultado convertido a `TriRaycastSample` | Medir coste real de raycasts y otras consultas; no "optimizar" lo ya optimizado |
| Geometría de colisión | [`physical_collision_world.rs`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/vehicle-physics-engine/src/physical_collision_world.rs) | Consultas de superficie, formas y colisiones físicas | Instrumentar por categoría, conservar colisiones reales |
| Rig heredado de 12 raycasts | [`game/scripts/vehicle/rust_vehicle_controller.gd`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/scripts/vehicle/rust_vehicle_controller.gd) | Expone FL/FR/RL/RR × inner/center/outer | Separar rutas legacy/player/acoplada; no romper compatibilidad |
| Puente Godot↔mundo físico | [`physical_vehicle_world_controller.gd`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/scripts/runtime/physical_vehicle_world_controller.gd) | Registra vehículos, envía inputs, avanza una vez por frame, publica snapshots | Punto de integración; no actualizar un mundo por coche |
| Referencia de Fuji | [`game/tracks/fuji76_77/metadata/racing_line.json`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/tracks/fuji76_77/metadata/racing_line.json) | Tiene `lap_length_m`, 989 waypoints, anchuras laterales, distancia de vuelta y branch | Utilizar como primer circuito; validar orientación/cierre/wrap |
| Composición de carrera actual | [`game/scenes/runtime/race_session.gd`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/scenes/runtime/race_session.gd) | Composición de sesión orientada a un vehículo activo | Añadir gestor de parrilla sin destruir modo de jugador único |
| Stub inicial de IA | [`game/crates/formula90-core/src/modules/ai_stub.rs`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/formula90-core/src/modules/ai_stub.rs) | No-op determinista; contiene futuros overrides de velocidad/acelerador | Conservar compatibilidad de módulo, **descartar idea de rubber-band y overrides macro** |
| Tests Rust de mundo acoplado | [`coupled_vehicle_world_test.rs`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/vehicle-physics-engine/tests/coupled_vehicle_world_test.rs) | Punto de partida para pruebas 1/4 coches, rendimiento, colisión y snapshot | Ampliar sin eliminar tests de regresión |
| Audio worker | [`game/crates/formula90-core/src/audio_worker.rs`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/formula90-core/src/audio_worker.rs) | Mezclador/desacoplo de audio ya existente | Añadir presupuestos y culling por vehículo; evitar un mixer completo por IA |
| Normas de contribución | [`AGENTS.md`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/AGENTS.md) y [`game/AGENTS.md`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/AGENTS.md) | BUILD/HEAD, cambios atómicos, Rust como dueño de simulación, nombres explícitos | Obligatorio respetarlas antes de generar código |
| Contrato y pruebas de integración | [`docs/coupled_vehicle_godot_integration.md`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/docs/coupled_vehicle_godot_integration.md) | Describe instancias físicas y limitaciones de aceptación | No asumir que la parrilla ya rinde bien; usar las pruebas existentes |

**Observación crítica:** en `coupled_vehicle_world.rs` también existen varias sondas aerodinámicas, consultas de contacto de neumático/carrocería y un solve de suspensión acoplada. El número de rayos no representa por sí solo el coste físico. Si la física acoplada tiene cuellos de botella en solvers/contactos, sustituir 12 por 4 sensores **no** resolverá el problema.

**Condición de trabajo para agente:** antes de modificar cualquier archivo inspeccionar la revisión actual, las instrucciones `AGENTS.md` aplicables, el código llamante/consumidor y las pruebas. Las rutas anteriores son puntos de partida verificados al redactar este plan, no un diff ni garantía de que sus contratos permanezcan invariables.

## 2. Alcance / principios funcionales

### 2.1 Dentro del alcance inicial

- Coche IA completamente autónomo que usa neumáticos, motor, aerodinámica, frenos, caja y contacto físico reales del simulador.
- Controles válidos: acelerador, freno, dirección, selección de marcha y señales auxiliares estrictamente necesarias.
- Uso de una **única** racing line de Fuji, con perfil de velocidad calculado offline o de forma determinista.
- Modo headless reproducible y escena visual de desarrollo.
- Cuatro vehículos registrados en un único mundo autoritativo, gestionados con la misma secuencia de ticks.
- Seguimiento de oponentes sin contacto intencional antes de añadir adelantamientos.
- Sonido de motor limitado para IA, con posibilidad de tener silenciadas IA lejanas.
- Telemetría por coche + informe agregado CSV/JSON.

### 2.2 Fuera de alcance para M0–M2

- Aprendizaje por refuerzo, redes neuronales y entrenamiento costoso.
- Director que fuerce lucha rueda a rueda o resultados de carrera.
- Adelantamientos antes de comprobar estabilidad de cuatro coches.
- Estrategia compleja de pit stops, Safety Car, clima o banderas en las primeras dos entregas.
- Física enteramente nueva exclusiva de IA: solo una versión del mismo modelo, **si** hay evidencia de rendimiento que lo justifique.
- Audio multicanal completo para todos los coches.
- Suponer que "raycast = colisión de carrocería": se mantienen dominios separados.

### 2.3 Arquitectura deseada

```text
                       RACE WORLD SNAPSHOT (Rust)
                  ┌───────────────────────────────┐
                  │ Estado físico de toda la carrera│
                  └──────────────┬────────────────┘
                                 │
                    ┌────────────┴────────────┐
                    ▼                         ▼
          SHARED PREDICTION           STRATEGIC RACE DIRECTOR
        posiciones / conflictos       baja frecuencia + eventos
        geometría / grip pista        predicción de degradación,
                    │                ritmo, tráfico, boxes
                    │                         │
                    │               OBJECTIVES + VALIDITY
                    │                         │
                    └────────────┬────────────┘
                                 ▼
                  LOCAL DRIVER [una instancia/coche]
                    Safety / Rules gate (veto)
                    Perception / local confidence
                    Utility evaluator (intención)
                    Racecraft finite state machine
                    Local candidate trajectory planner
                    Longitudinal + lateral controllers
                                 │
                    VehicleControlRequest por tick
                                 ▼
                       RUST PHYSICAL WORLD
                                 │
                          next snapshot
```

**Claridad de autoridad:** compartir un predictor entre pilotos no significa que la IA macro ordene ataques. Los pilotos pueden usar el mismo servicio de cálculo, con vistas y niveles de conocimiento apropiados a sus circunstancias. El mundo físico no obedece al director estratégico: obedece a controles locales con límites físicos.

### 2.4 Contratos y estados

- `RaceWorldSnapshot`: identificador de tick, tiempo fijo, identificador de vehículo, pose, velocidad, aceleraciones, orientación, ruedas, grip estimado, daños, neumáticos, posición de carrera y eventos; contiene fuentes autoritativas, no inferencias.
- `TrackPathSample`: distancia longitudinal `s`, centro del carril o racing line, tangente, curvatura, anchuras disponibles, cota, normal, material y velocidad recomendada; definir convención de ejes explícita.
- `LocalDriverObservation`: snapshot filtrado de rivales cercanos, incertidumbre, ventana de lookahead, marcadores de curva y objetivo vigente.
- `DriverObjective`: ID, tipo (`MAINTAIN_PACE`, `PUSH_PACE`, `PRESERVE_TYRES`, `PIT_THIS_LAP`, `ATTACK_TARGET`, `HOLD_POSITION`), vehículo objetivo opcional, caducidad, razón, restricciones, revisión; objetivo, **no** input físico.
- `ObjectiveFeedback`: `ACCEPTED`, `DEFERRED`, `REJECTED`, `COMPLETED`, razón, timestamp, revisión. Nunca bloquear control local a la espera de respuesta macro.
- `DriverTacticalState`: `FOLLOW_RACING_LINE`, `FOLLOW_OPPONENT`, `PREPARE_ATTACK`, `COMMIT_ATTACK`, `SIDE_BY_SIDE`, `CLEAR_OPPONENT`, `RETURN_TO_LINE`, `YIELD_OR_ABORT`, `RECOVERY`, `PIT_ENTRY`.
- `VehicleControlRequest`: dirección/acelerador/freno normalizados y marcha solicitada; `vehicle_identifier`, `physics_tick`, origen (`human`/`autonomous`) y validez; fuente única por entidad/tick.
- **Revisión y TTL:** las órdenes vencidas no pueden tomar control. Si la IA macro no responde, la local sigue conduciendo con seguridad y su último objetivo válido o comportamiento por defecto.

> Nombres de tipos aquí son propuestas. En el código nuevo, aplicar la política de nombres autoexplicativos de `AGENTS.md`, sin símbolos abreviados ni nuevos comentarios de código.

## 3. Estructura de carpetas propuesta (mínima y compatible con canon)

Se recomienda implementar **decisión y controlador en Rust** por cercanía al mundo físico y menor cruce de FFI. El **C++ nativo existente** conserva el papel de integración con Godot, y **GDScript** la composición/depuración. Esto actualiza una propuesta genérica C++/Rust a la realidad del repositorio, donde Rust ya es dueño de simulación. No introducir una segunda verdad física. Verificar la topología real de `game-sim` y `formula90-core` antes de fijar un nuevo crate.

```text
formula-90/
├── docs/
│   └── race-intelligence/
│       ├── AGENTS.md
│       ├── architecture.md
│       ├── decision-contracts.md
│       ├── physics-performance-baseline.md
│       ├── milestone-reports/
│       └── reference-attribution.md
├── game/
│   ├── crates/
│   │   └── race-intelligence-engine/        # crate Rust PROPUESTO
│   │       ├── AGENTS.md
│   │       ├── Cargo.toml
│   │       ├── src/
│   │       │   ├── lib.rs
│   │       │   ├── world_state.rs
│   │       │   ├── track/
│   │       │   │   ├── mod.rs
│   │       │   │   ├── path_reader.rs
│   │       │   │   ├── path_projection.rs
│   │       │   │   └── speed_profile.rs
│   │       │   ├── local_driver/
│   │       │   │   ├── mod.rs
│   │       │   │   ├── observation.rs
│   │       │   │   ├── safety_supervision.rs
│   │       │   │   ├── steering_control.rs
│   │       │   │   ├── longitudinal_control.rs
│   │       │   │   ├── gear_selection.rs
│   │       │   │   ├── opponent_tracking.rs
│   │       │   │   ├── behavior_selection.rs
│   │       │   │   ├── racecraft_state_machine.rs
│   │       │   │   └── trajectory_selection.rs
│   │       │   ├── shared_prediction/
│   │       │   │   ├── opponent_trajectories.rs
│   │       │   │   └── spatial_index.rs
│   │       │   ├── strategy/
│   │       │   │   ├── race_director.rs
│   │       │   │   ├── objective_dispatcher.rs
│   │       │   │   ├── pit_decisions.rs
│   │       │   │   ├── degradation_model.rs
│   │       │   │   └── outcome_estimator.rs
│   │       │   ├── telemetry/
│   │       │   │   ├── metrics.rs
│   │       │   │   └── decision_trace.rs
│   │       │   └── scheduling.rs
│   │       ├── tests/
│   │       │   ├── line_following_test.rs
│   │       │   ├── opponent_following_test.rs
│   │       │   ├── passing_decision_test.rs
│   │       │   ├── objective_priority_test.rs
│   │       │   └── deterministic_replay_test.rs
│   │       └── benches/
│   │           └── multi_vehicle_benchmark.rs
│   ├── data/
│   │   ├── race_intelligence/
│   │   │   ├── AGENTS.md
│   │   │   ├── driver_profiles.json
│   │   │   ├── control_profiles.json
│   │   │   ├── physics_lite_profiles.json
│   │   │   └── race_director_rules.json
│   │   └── tracks/fuji76_77/            # EVITAR duplicar fuentes: usar metadata existente
│   ├── scripts/runtime/
│   │   ├── race_grid_controller.gd       # composición y registro, NO física paralela
│   │   └── race_intelligence_bridge.gd   # comando visual/test, si hace falta
│   ├── scenes/runtime/
│   │   ├── autonomous_lap_test.tscn
│   │   └── four_vehicle_grid_test.tscn
│   └── tests/
│       ├── test_autonomous_lap.gd
│       ├── test_four_vehicle_grid.gd
│       └── test_ai_audio_budget.gd
├── tools/
│   └── race_intelligence/
│       ├── AGENTS.md
│       ├── benchmark_runner.py
│       ├── metrics_report.py
│       ├── plot_racing_line.py
│       └── scenarios/
│           ├── one_driver_fuji.json
│           ├── four_drivers_following.json
│           ├── four_drivers_overtaking.json
│           └── safety_cases.json
└── scratch/race_intelligence/          # resultados no promocionados
    ├── raw_telemetry/
    ├── benchmark_results/
    └── captures/
```

**Directorio vs convención:** `game/` usa `AGENTS.md` por dominio. Crear los `AGENTS.md` requeridos para cada carpeta nueva según normas locales. Los `.json` de líneas de carrera y perfiles físicos canónicos existentes siguen siendo la fuente de verdad; **no duplicarlos** bajo una ruta propuesta. Las rutas de escenas `.tscn` son nuevas; comprobar primero escenas actuales y nombres efectivamente usados.

**Archivos existentes a integrar, no reemplazar a ciegas:** `game/crates/formula90-core/src/modules/ai_stub.rs`, `game/crates/vehicle-physics-engine/src/coupled_vehicle_world.rs`, `game/scripts/runtime/physical_vehicle_world_controller.gd`, `game/scenes/runtime/race_session.gd`, `game/scripts/vehicle/rust_vehicle_controller.gd`, `game/crates/formula90-core/src/audio_worker.rs`.

## 4. P0 — Auditoría de física, un raycast por rueda y rendimiento (BLOQUEANTE)

**Objetivo P0:** determinar cuánto cuesta cada vehículo autónomo, asegurar un solo mundo físico y establecer un perfil de carga realmente sostenible. **No comenzar un sistema de adelantamientos hasta aprobar P0 y M2.**

### P0-01 — Instrumentación y baseline

- [ ] Registrar rama, HEAD, `git status`, `BUILD_SOURCE`, build mode, CPU/GPU, frecuencia de física, resolución, renderizador, audio activo y parámetros físicos.
- [ ] Reproducir **escena de prueba sin IA**, jugador controlado por reproducción de inputs fijos; comparar 1, 2 y 4 vehículos registrados, además de 4 vehículos inactivos sobre pista y 4 avanzando.
- [ ] Instrumentar tiempo por etapa del mundo Rust: avance de fuerzas, `raycast_driveable`, sondas aerodinámicas, búsqueda de contactos de formas, narrow phase, solver acoplado, colisiones entre entidades, snapshot serialización, puente Godot.
- [ ] Contar **consultas realmente ejecutadas** por tick y por coche, no solo nodos `RayCast3D` presentes en escena; separar rayos rueda, sondas aero, casts/collisions geométricos.
- [ ] Repetir medición audio apagado, solo motor, audio completo de jugador; con/sin renderización, física misma configuración.
- [ ] Ejecutar microbenchmarks (10 000 ticks tras 1 000 de calentamiento) y carrera 10 min equivalente; guardar mediana/p95/p99 y máximo de cada etapa, memoria pico, número de vehículos, colisiones y contactos.
- [ ] Demostrar que la integración no invoca un `advance` por vehículo; solo una llamada por mundo y paso, con inputs sellados para todas las entidades antes del avance.
- [ ] Generar `docs/race-intelligence/physics-performance-baseline.md` con tablas y evidencia reproducible.

**Referencias actuales:** [CoupledVehicleWorld](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/vehicle-physics-engine/src/coupled_vehicle_world.rs), [colisión Rust](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/vehicle-physics-engine/src/physical_collision_world.rs), [world bridge Godot](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/scripts/runtime/physical_vehicle_world_controller.gd), [CARLA Traffic Manager y sincronización por etapas](https://github.com/carla-simulator/carla/blob/ue5-dev/Docs/adv_traffic_manager.md).

**Aceptación:** `baseline.json` con HEAD, entorno y métricas de 1/2/4 coches; sin errores ni NaN; p95/p99 reportados; costes clasificados por módulo. Sin números de referencia, **ninguna optimización se declara eficaz**.

### P0-02 — Contrato de `AI_LITE` (solo si la medición lo necesita)

- [ ] Crear `physics_lite_profiles.json` como **configuración de opt-in**, no como nueva física independiente; definir modo `FULL`/`AI_LITE` y trazar qué funciones cambian.
- [ ] Garantizar **una única muestra efectiva de terreno por rueda** en `AI_LITE`, con `RaycastHit` posición/normal/material/distancia estable. El camino Rust actual ya rellena `inner=center=outer` con una muestra por rueda: verificar si basta sin introducir otro rig.
- [ ] Evitar que nodos legacy de raycast hagan actualizaciones redundantes cuando un coche está bajo control del mundo Rust; preservar esos sensores cuando se usa la modalidad legacy que los requiere.
- [ ] Mantener soporte de muelle/amortiguador, fuerzas verticales y laterales/longitudinales, slip, reparto de masas, aerodinámica, motor, caja, frenos y masa del vehículo. **La IA nunca usa física cinemática falsa.**
- [ ] Inspeccionar la frecuencia y necesidad de: térmicas, desgaste, geometría de suspensión, fuerzas por contacto, sondas de fondo plano y fricción por superficie. Reducir la frecuencia de subsistemas **no críticos** solo si se demuestra estabilidad y equivalencia aceptable. Nunca saltar integración dinámica ni chequeos necesarios de colisión.
- [ ] Evaluar `AI_LITE` contra `FULL` en recta, frenada, curva sostenida, pianos, bache y transición suelo/pista. Usar inputs idénticos, condiciones iniciales iguales y comparaciones de trazas.
- [ ] Para la futura parrilla mixta, definir política de justicia: (A) todos los coches participantes utilizan el mismo perfil de contacto físico, **preferido**, o (B) se acepta un `AI_LITE` diferenciado **solo** tras evidencia cuantitativa de paridad y sin ventajas de adherencia/choque. Documentar el riesgo.
- [ ] Preservar colisiones reales entre vehículos y barreras incluso en nivel LITE. **No** convertir los cuatro rayos en sustitutos de colisiones volumétricas.
- [ ] Validar que las físicas del jugador no cambian cuando se habilita/deshabilita la IA.

**Código a inspeccionar:** [mundo multivehículo](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/vehicle-physics-engine/src/coupled_vehicle_world.rs), [suspensión acoplada](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/vehicle-physics-engine/src/suspension_coupled_solver.rs), [neumáticos](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/vehicle-physics-engine/src/tire.rs), [legacy Godot 12 raycasts](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/scripts/vehicle/rust_vehicle_controller.gd). [Godot RayCast3D — actualización por tick](https://docs.godotengine.org/en/stable/classes/class_raycast3d.html).

**Umbrales de paridad iniciales (ajustables únicamente con evidencia):** misma pista y setup, diferencia de tiempo por vuelta mediano ≤3%; velocidad al salir de curva ≤5%; distancia de frenado 200→80 km/h ≤5%; sin inversión de orden de rendimiento causada por el perfil; cero errores de contacto o pérdida de suelo persistente por muestreo central. Tomar al menos **10 vueltas medidas por perfil** y ≥5 repeticiones por escenario físico controlado. Si falla, **no aprobar AI_LITE**: conservar FULL e investigar otro cuello de botella.

### P0-03 — Cadencia y determinismo físico

- [ ] Respetar el mínimo de frecuencia que valida el mundo acoplado actual: en `CoupledVehicleWorldConfiguration` se admiten valores **desde 120 Hz** (hasta 4 000 Hz según el contrato actual). **No reducir a 60 Hz sin cambiar formalmente ese contrato y validar estabilidad/regresiones**.
- [ ] Implementar separación explícita entre tick físico (p.ej., 120 Hz), control (p.ej., 60–120 Hz) y decisiones tácticas (5–10 Hz); mantener último control válido entre decisiones.
- [ ] Usar un orden estable por `vehicle_identifier`; producir mismo resultado con mismo estado, comandos, configuración, seed y tick fijo. No usar `delta` visual para decisiones físicas.
- [ ] Medir desincronización `input timestamp` vs `physics_tick`; registrar errores de comando vencido o duplicado.
- [ ] Documentar cómo influye render/pausa en tiempo físico. El modo headless y visual deben compartir configuración física.

**Aceptación:** 3 ejecuciones con mismo escenario y seed reproducen checksum exacto de decisiones y de snapshots normalizados en el mismo entorno/build; si diferencias de coma flotante impiden bit exacto entre plataformas, documentar tolerancias cuantitativas por campo, conservando determinismo exacto en un mismo binario/máquina.

### P0-04 — Audio motor-only para IA (parte de P0)

- [ ] Crear control de calidad acústica por entidad: `PLAYER_FULL`, `AI_ENGINE_ONLY`, `AI_SILENT` o equivalente con nombres autoexplicativos.
- [ ] Fuente de motor: RPM, throttle, load y estado de encendido **desde snapshot físico real**, nunca oscilador/animación independiente.
- [ ] Para `AI_ENGINE_ONLY`, usar **un único reproductor lógico de motor por coche audible**; si el banco existente necesita internamente crossfade de capas, presupuestarlo como una voz lógica y medir número de voces DSP reales. No prometer que un solo sample satisface todo el rango de RPM.
- [ ] Deshabilitar por defecto para IA: efectos de engranajes, whine separado, plank, tire scrub, ambiente individual, pops/explosiones, impactos sonoros, partículas acústicas y telemetría innecesaria; solo deshabilitar **audio**, no daños o colisiones de física.
- [ ] Definir distancia de inicio/parada de audición con histéresis; mezclar prioritariamente las 2 IA más audibles/visibles en el escenario de 4 coches. Las restantes pueden permanecer sin voz activa, pero RPM/caja/estado siguen avanzando en la simulación.
- [ ] Limitar actualización de parámetros acústicos de IA (por ejemplo, 20–30 Hz); interpolar parámetros dentro de audio sin bloquear render o física.
- [ ] Verificar que audio del jugador **no** baja calidad cuando aparecen IA y que no hay creación/destrucción de voz cada frame.
- [ ] Medir CPU DSP, buffers, XRuns/underruns, voces activas y memoria del banco.

**Código actual:** [audio worker Rust](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/formula90-core/src/audio_worker.rs), [motor de audio](https://github.com/Anthony38zrgo-2/formula-90/tree/main-clean/game/crates/vehicle-audio-engine), [audio façade](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/formula90-core/src/audio.rs), [sonidos runtime canónicos](https://github.com/Anthony38zrgo-2/formula-90/tree/main-clean/game/sounds).

**Aceptación:** en cuatro coches, por defecto 1 voz principal del jugador + **máximo 2 voces lógicas activas de IA**; nunca se crean streams por frame; cero underruns durante prueba de 10 min; p95 CPU DSP reportado y sin regresión >15% frente al baseline aceptado con la misma cantidad de voces. El número "2 IA audibles" es un presupuesto inicial, no un límite perceptual permanente.

### P0-05 — Hito de rendimiento P0 (GO/NO-GO)

- [ ] Guardar configuración hardware de referencia y baseline de FPS de escena sin IA.
- [ ] Con 4 coches físicos reales y misma pista, objetivo inicial en el PC de referencia: **60 FPS p95 de tiempo de frame ≤16,7 ms** bajo preset fijo (recomendar 1080p); registrar FPS 1% low y máximos.
- [ ] Para 120 Hz de física: presupuesto de tick físico p95 **≤6,5 ms** (de 8,33 ms disponibles, con margen). El tiempo aquí incluye **todos los 4 vehículos** y las colisiones, no cada vehículo independientemente.
- [ ] Reserva inicial del sistema de decisión (4 IA): p95 de trabajo combinado ≤**1,5 ms por frame de render equivalente**, medido aparte de física; si se programa cada 2/6/12 ticks, indicar coste por ejecución y amortizado.
- [ ] Confirmar ausencia de crecimiento de memoria a lo largo de 10 min de simulación; umbral inicial de incremento residual ≤5% tras warmup, excluyendo caches preasignadas documentadas.
- [ ] Comparar FULL vs LITE con **idénticos** escenarios: publicar porcentajes de ahorro y compromisos de paridad. Se admite no usar LITE si FULL ya pasa todas las puertas.

> Las cifras son **objetivos de aceptación propuestos**, no resultados medidos. Si el equipo objetivo rinde distinto, negociar y registrar presupuesto sobre hardware reproducible, no esconder regresiones alterando la escala de tiempo.

**Entregable P0:** `physics-performance-baseline.md`, benchmark automatizado 1/2/4 coches, perfil físico documentado, ruta de audio reducida y decisión justificada `FULL` o `AI_LITE`.

### P0-06 — Estabilidad de amortiguación, topes y contactos a 120 Hz (BLOQUEANTE ACTUAL)

**Estado:** corrección parcial implementada; banco de amortiguación aprobado, conducción cerrada y Fuji todavía fallan. Ver corte 0.1.

- [ ] Reproducir y corregir el fallo de solapamiento en `closed_loop_flat_test` sin ampliar dominios de recuperación, tolerancias de penetración o presupuestos para ocultarlo.
- [ ] Separar contribuciones de amortiguación, restricciones de recorrido, impulsos de colisión, correcciones de posición y mandos de dirección; medir energía, carga por rueda, inclinación y contactos.
- [ ] Validar el localizador de topes con el paso implícito, entradas/salidas de topes, frenada fuerte y apoyo asimétrico. Conservar resultados de los bancos que ya pasan.
- [ ] Demostrar estabilidad y convergencia del conjunto a 120 Hz; los pasos más pequeños son referencias de diagnóstico, no un cambio silencioso de frecuencia de producción.
- [ ] Reconstruir y verificar el último fuente/DLL, repetir smoke canónico y una vuelta real de Fuji sin límite global antes de retomar la calibración de IA.

**Aceptación:** banco pasivo y excitación seguida de asentamiento sin oscilación autosostenida; prueba cerrada de suelo plano aprobada; cero errores de dominio de colisión/solapamiento en el escenario nominal de Fuji; trazas reproducibles y análisis del balance energético, incluido el transitorio.

### P0-07 — Replay de diagnóstico y contrato numérico Godot→Rust

**Estado:** herramienta de comparación creada, ejecución bloqueada por tipos numéricos JSON.

- [ ] Corregir el transporte de campos enteros serializados como flotantes por Godot, sin aceptar valores fraccionarios o fuera de rango ni debilitar el contrato de producción.
- [ ] Reproducir los mismos inputs registrados sobre el mismo paquete físico, con estado inicial y configuración documentados, sin realimentación de una IA distinta en cada ejecución.
- [ ] Comparar carga, giro, recorrido, energía, eventos y trayectoria entre cadencias; registrar fallos y desviaciones antes de concluir si un comportamiento es físico, numérico o del controlador.

**Aceptación:** replay que llega a ejecutar la física, sin errores de deserialización, y reporte de convergencia real. Un test que falla antes de avanzar no aporta evidencia de convergencia.

## 5. M1 — Primera IA autónoma: correr sola por la racing line (PRIMER HITO JUGABLE)

**Estado del hito:** implementación candidata parcial; **NO APROBADO**. La prioridad vigente es P0-06/P0-07. Las casillas siguientes son criterios de aceptación, no una negación del código ya existente descrito en 0.1.

**Escenario:** Fuji 1976–77, 1 coche F1 2030, pista vacía, sin director estratégico ni rivales. Debe arrancar, seguir trayectoria, frenar, acelerar, cambiar de marcha y completar vueltas sin intervención.

### M1-01 — Adaptador de racing line y topología

**Avance:** lector/proyector y validación geométrica implementados en `race-intelligence-engine/src/track/`; overlay completo y aceptación sostenida pendientes.

- [ ] Leer el JSON canónico de Fuji y validar esquema y `waypoint_count`, anchuras, branch y `lap_distance_m`.
- [ ] Ordenar/cerrar el anillo según geometría/distancia física: **no suponer que el primer waypoint tiene `s=0`**; el JSON observado comienza cerca del final de vuelta. Comprobar wrapping, segmento cruzando línea de meta, punto de spawn y orden correcto.
- [ ] Proyectar posición del vehículo a Frenet simplificado (`s` longitudinal, `d` lateral) o coordenadas equivalentes; identificar segmento mediante última proyección válida y búsqueda local para evitar saltos de rama.
- [ ] Filtrar puntos duplicados, ceros de distancia y curvatura ruidosa; interpolar tangentes, curvatura y elevación.
- [ ] Añadir validaciones de anchura útil incluyendo **semiancho del vehículo + margen**, no solo distancia del centro a bordes.
- [ ] Producir overlays de racing line, índice, punto lookahead, ancho de pista y trayectoria predicha.

**Implementación sugerida:** `track/path_reader.rs`, `track/path_projection.rs`.<br>
**Datos existentes:** [Fuji racing line](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/tracks/fuji76_77/metadata/racing_line.json); [TUM global optimization](https://github.com/TUMFTM/global_racetrajectory_optimization/blob/master/main_globaltraj.py); [helpers de trayectoria](https://github.com/TUMFTM/trajectory_planning_helpers).

**Aceptación:** 100% waypoints válidos, sin segmentos de longitud cero en trayectoria usada, `s` continuo durante 3 vueltas; sin saltos a un tramo vecino en horquillas ni signo de curvatura invertido.

### M1-02 — Perfil de velocidad basado en física

**Avance:** envolvente nominal del coche, curvatura, pendiente y pasada de frenado implementadas. Objetivo longitudinal en posición actual; anticipación de dirección desacoplada del frenado. Pendiente validación dinámica; no declarar completa la pasada de aceleración ni la adaptación al grip en vivo.

- [ ] Generar límite de curva a partir de curvatura/agarre lateral estimado; empezar con un perfil **conservador** y multiplicador calibrable, no velocidad fija global.
- [ ] Añadir pasada hacia atrás de límite de frenado (deceleración máxima) y pasada hacia delante de aceleración disponible; incorporar curvas, cambios de rasante y grip local.
- [ ] Para la aerodinámica, evitar estimar toda la fuerza lateral con fórmula plana constante: usar límite aproximado dependiente de velocidad/calibración cuando sea necesario.
- [ ] Guardar perfil derivado reproducible con digest del JSON fuente, digest configuración física y versión del generador; recalcular cuando cambie circuito o coche.
- [ ] Testear curva estrecha y salida de curva con perfil no oscilatorio; contar sobrepasos de target y frenada tardía.

**Referencia:** [TUM minimum-time optimization](https://github.com/TUMFTM/global_racetrajectory_optimization/tree/master/opt_mintime_traj), [VDrift perfil de velocidad y frenado](https://github.com/VDrift/vdrift/blob/master/src/ai/ai_car_experimental.cpp).

**Aceptación:** límite de curva/capacidad de frenar respetado con margen en ≥99% de muestras en 10 vueltas; no superar velocidad físicamente factible por error del planificador; trazado de velocidad objetivo vs real disponible.

### M1-03 — Control lateral

**Avance:** seguimiento geométrico, calibración de cremallera, límite de variación y recuperación lateral continua implementados. Respuesta con oscilaciones, pérdidas de apoyo y salidas todavía no aprobada; separar P0-06 de la calibración del controlador.

- [ ] Implementar seguidor de lookahead dependiente de velocidad (`Pure Pursuit` como baseline) y corrección por error lateral/heading. Para alta velocidad estudiar Stanley u otro controlador con feedforward de curvatura.
- [ ] Limitar volante por velocidad, slip, yaw rate y rapidez de cambio realista; no configurar steer instantáneo imposible.
- [ ] Manejar parada/casi parada con fallback para no dividir por velocidad cero.
- [ ] Registrar `s`, `d`, error de heading, volante pedido, ángulo obtenido y saturación.
- [ ] Crear test de recuperación tras perturbaciones pequeñas: desplazamiento inicial lateral y orientación desviada, sin teletransporte.

**Referencia:** [ForzaETH Pure Pursuit/controller](https://github.com/ForzaETH/race_stack/tree/main/controller/pp), [VDrift controlador estándar](https://github.com/VDrift/vdrift/blob/master/src/ai/ai_car_standard.cpp), [VDrift controlador experimental](https://github.com/VDrift/vdrift/blob/master/src/ai/ai_car_experimental.cpp).

**Aceptación:** error lateral absoluto **p95 ≤1,0 m** en condiciones normales, máximo ≤2,5 m en 10 vueltas de referencia; cero salidas donde la anchura no lo justifique; si falla en sectores concretos registrar sector/speed y corregir controlador, no inflar agarre.

### M1-04 — Control longitudinal, caja y arranque

**Avance:** pedales con anticipación por perfil, anti-windup y banda de seguimiento; selección automática y corrección del bloqueo al corte implementadas. Faltan validación sostenida, suavidad y condiciones anómalas de arranque/daños.

- [ ] Convertir perfil objetivo en aceleración/frenado con PID o feedforward + realimentación, anti-windup, deadband e histéresis throttle/brake.
- [ ] Anticipar distancia de frenada con velocidad propia y grip actual; no esperar a `v_actual > v_target` en ápice para empezar a frenar.
- [ ] Implementar marcha automática para IA según RPM, relación de marchas, curva de par y límites de cambio; evitar cambios repetidos alrededor de umbral.
- [ ] Gestionar inicio de sesión: encendido, embrague/launch si corresponde, marchas 1–7, velocidad cero, motor calado y degradación de rendimiento por daños.
- [ ] Nunca alterar `VehicleInput` con potencia ficticia, velocidades objetivo como fuerza externa o teletransporte.

**Referencia:** [TORCS tutorial control de robots](https://torcs.sourceforge.net/api/robot_tutorial.html), [VDrift gestión gas/freno](https://github.com/VDrift/vdrift/blob/master/src/ai/ai_car_experimental.cpp), [Powertrain Rust existente](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/vehicle-physics-engine/src/powertrain.rs).

**Aceptación:** 10 vueltas de principio a fin sin intervención, sin marcha inválida, sin oscilaciones rápidas de acelerador/freno (medir variación por segundo), con target speed y actual graficables; **0 input NaN/Inf**.

### M1-05 — Control autoritativo + escena de demo

**Avance:** ejecutable local de control y harness Godot opt-in disponibles, con telemetría y runner. La escena/selector de producto y el cambio de dueño humano/IA completo siguen pendientes.

- [ ] Crear escena `autonomous_lap_test.tscn`, separada del modo de carrera de jugador; activar/desactivar piloto automático desde un selector.
- [ ] Integrar IA con arbitraje de input **antes** de `advance` y después de snapshot anterior; no modificar directamente posición o velocidad física.
- [ ] Definir cambio seguro de dueño `HUMAN -> AUTONOMOUS` solo con handshake de un tick; evitar doble escritura en `vehicle.steering_input`, `throttle_amount`, `brake_amount`.
- [ ] Mantener `RaceSession` convencional funcionando para usuario humano; no cambiar contratos de HUD/audio del jugador.
- [ ] Hacer visible telemetría: objetivo, actual, error lateral, zona, marcha, ritmo vuelta, estado AI, frecuencia y costes.

**Código existente:** [world bridge](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/scripts/runtime/physical_vehicle_world_controller.gd), [session](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/scenes/runtime/race_session.gd), [stub de IA](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/formula90-core/src/modules/ai_stub.rs).

### PUERTA M1 (obligatoria)

- [ ] En **3 seeds** y **dos reinicios** por seed, el coche completa **≥10 vueltas consecutivas por ejecución** en Fuji sin reset externo ni salida de pista que invalide la vuelta.
- [ ] Ninguna colisión con barrera por error de trayectoria; pueden documentarse eventos de roce controlado con pianos. En el ensayo vigente sin límite global, corregir el perfil físicamente factible y los fallos de contacto/control; no introducir un límite global para ocultarlos. Las pruebas históricas limitadas son controles separados y no aprueban esta ejecución.
- [ ] `p95(|d|) ≤1,0 m`; cero NaN/Inf; contador de vueltas correcto; variación de tiempo por vuelta sin tráfico, después de warmup, coeficiente de variación ≤3% con mismo coche/condiciones.
- [ ] Gráficos `s`, error lateral, v_objetivo/v_real, aceleración, freno y steering, capturas + registro headless.
- [ ] Se ejecuta con el mismo integrador y neumáticos que el coche controlado por humano (salvo perfil LITE **medido y aceptado**).

**Entrega M1:** **primera IA funcional visible** y test de vueltas solo. No agregar perfil de agresividad ni macro decisiones hasta finalizar esta entrega.

## 6. M2 — Parrilla de cuatro: estabilidad, coste y seguimiento básico

**Escenario:** cuatro F1 2030 de una misma física autoritativa, mismas reglas y velocidades de referencia ligeramente distintas, grid espaciado; primero run ordenado **sin adelantar**.

### M2-01 — Composición y spawn

- [ ] Extender composición para `0–4` IA más jugador si se requiere, sin perder `RaceSession` actual; crear `race_grid_controller.gd` y escena `four_vehicle_grid_test.tscn`.
- [ ] Registrar todas las entidades una sola vez mediante interfaz Rust; garantizar identificadores únicos, perfiles válidos, matrices físicas y snapshots separados.
- [ ] Definir 4 posiciones reales de parrilla con orientación sobre tangente de racing line y holguras/alternancia izquierda-derecha; validar que al aparecer no se superponen volúmenes.
- [ ] Enviar controles de todos los coches antes de avanzar el mundo; snapshots de mismo tick; resultados independientes del orden de registro.
- [ ] Implementar launch escalonado o salidas sincronizadas según escenario, pero sin modificar parámetros físicos para favorecer a un coche.

**Referencia propia:** [world controller](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/scripts/runtime/physical_vehicle_world_controller.gd). **Inspiración:** [CARLA Traffic Manager](https://github.com/carla-simulator/carla/blob/ue5-dev/LibCarla/source/carla/trafficmanager/TrafficManagerLocal.cpp).

**Aceptación:** 4 entidades únicas, spawns válidos, un solo `advance` físico por tick, telemetría por coche, sin colisión en primer frame y sin renderizar representaciones fantasmas de un quinto coche.

### M2-02 — Percepción compartida de rivales

- [ ] Crear una tabla central de poses/velocidades relativas por tick e índice espacial, reutilizable, con vistas locales filtradas.
- [ ] Asociar oponentes por distancia longitudinal y lateral sobre pista, **no** por distancia euclidiana aislada (horquillas y segmentos cercanos).
- [ ] Clasificar `FRONT`, `BACK`, `SIDE`, `OVERLAP`, `PIT`, `OFF_TRACK`; diferenciar coche una vuelta adelantado de un coche físicamente detrás.
- [ ] Calcular TTC longitudinal cuando velocidad relativa de cierre sea positiva; cubrir TTC infinito cuando no hay cierre; estimar ocupación lateral con anchura real del coche.
- [ ] Añadir distancia de seguridad adaptada a frenada, neumáticos, dirty air y latencia de decisiones; primer modo `FOLLOW`, sin rebasar.
- [ ] Compartir geometría y consultas caras, no estado interno de decisiones: cada piloto solo usa lo que le corresponde.

**Código:** [Speed Dreams — clasificación de rivales](https://github.com/argos-research/speed-dreams/blob/master/src/drivers/kilo2008/opponent.cpp), [CARLA CollisionStage](https://github.com/carla-simulator/carla/blob/ue5-dev/LibCarla/source/carla/trafficmanager/CollisionStage.cpp), [ForzaETH trailing](https://github.com/ForzaETH/race_stack/blob/main/state_machine/src/state_transitions.py).

**Aceptación:** tests de aproximación trasera, overlap lateral, rival detenido, horquilla, cruce meta, coches en pit lane; exactitud de clasificación ≥99% en **set etiquetado determinista de ≥200 casos**. Test de colisión no dependerá de TTC si ya hay overlap.

### M2-03 — Following estable y diferencial de ritmo

- [ ] Todos usan misma racing line y perfil base; crear 4 perfiles de ritmo/suavidad (`pace_multiplier` explícito **solo en target de conducción**, nunca en fuerza/grip físico).
- [ ] Si el de delante circula lento, el que sigue adapta target speed y distancia segura sin intentar atravesarlo.
- [ ] Al desaparecer el obstáculo, recuperar progresivamente el ritmo; no alternar `FOLLOW`/`CRUISE` en cada tick (histéresis temporal/espacial).
- [ ] Conservar seguridad incluso si un coche delantero frena antes de lo esperado; probar frenada sorpresa con margen realista y condiciones iniciales viables.
- [ ] Seguimiento sin invasión intencional de espacio lateral y sin bloqueo completo indefinido por un falso positivo de percepción.

**Código:** [Speed Dreams KDriver — oponentes y offset](https://github.com/argos-research/speed-dreams/blob/master/src/drivers/kilo2008/kdriver.cpp), [TORCS racing robots](https://torcs.sourceforge.net/api/robot_tutorial_chapter_5.html), [HighwayEnv seguimiento longitudinal](https://github.com/Farama-Foundation/HighwayEnv/blob/main/highway_env/vehicle/behavior.py).

**Aceptación:** 5 ejecuciones de ≥10 vueltas con 4 coches (≥200 vueltas-coche), **0 choques evitables por alcance** en condiciones nominales, 0 loops de estado, 0 resets, ritmo mediano por piloto medido. Colisiones inevitables provocadas explícitamente por un caso de prueba no se atribuyen al control nominal.

### M2-04 — Audio, FPS y perfiles de render

- [ ] Aplicar audio `AI_ENGINE_ONLY`; registrar voces lógicas/real DSP activas por entidad.
- [ ] Reducir animación/IK y actualizaciones visuales alejadas si son cuellos de botella, pero **no** física de colisión ni ritmo de control sin benchmark.
- [ ] Instrumentar draw calls, tiempo Godot script, tiempo audio, tiempo Rust, tiempo FFI, allocations, frame timing, colisión por coche.
- [ ] Ejecutar headless vs render 1080p y comparar coste; priorizar ahorro en shaders/IK/mezclador si la física ya pasa.
- [ ] Definir presupuesto de presentación de IA por distancia a cámara separado de presupuesto físico, para futura parrilla grande.

### PUERTA M2 (obligatoria)

- [ ] 4 coches completan en grupo 10 vueltas en 5 seeds, sin contacto por alcance nominal y sin abandono artificial de racing line.
- [ ] 60 FPS y p95 física ≤6,5 ms en hardware/preset fijados, o excepción técnica aprobada con plan de reducción y números exactos.
- [ ] 1+2 voces motor lógicas máximas por defecto con jugador; sin underrun; memoria estable.
- [ ] Mismo resultado en repetición determinista; grid/spawn/carreras reproducibles por seed.
- [ ] Comparación física FULL vs LITE publicada. No aumentar artificialmente velocidad para "facilitar" 4 coches.

**Entrega M2:** pequeña parrilla físicamente coherente que puede conducir en formación, con métricas continuas. **Es el primer prototipo válido de carrera multivehículo**, todavía sin adelantamientos.

## 7. M3 — Racecraft local: seguir, preparar, atacar y conducir rueda a rueda

**Dependencia:** M2 aprobado. La IA **no** recibe un "debes adelantar aquí" del estratega; detecta oportunidades por sí misma. La presencia de un objetivo macro puede cambiar prioridad de ataque, pero nunca su viabilidad.

### M3-01 — Mapa de oportunidades por sector

- [ ] Añadir metadatos derivados al circuito (archivo bajo dominio canónico del circuito, no duplicar racing line): anchura, longitud de frenada, sentido y radio de curva, velocidad, espacio útil para dos coches, posibilidad de terminar adelantamiento y riesgo de salida. Precálculo offline, revisable por diseñador.
- [ ] Dar a cada tramo atributo `overtaking_candidate` y dificultad continua, no simple permiso que invalide una batalla ya comenzada.
- [ ] Regla base para iniciar `PREPARE_ATTACK`: **(a) diferencial de ritmo/potencial real suficiente + (b) sector favorable** + (c) oportunidad física local plausible. Ambas condiciones mencionadas por diseño son necesarias para ataques normales.
- [ ] Tratar `SIDE_BY_SIDE` y `ABORT/HOLD` como problemas de seguridad **aunque el sector deje de ser favorable**. El coche no desaparece ni vuelve a la línea ideal repentinamente.
- [ ] Probar zona poco apta de alta velocidad (ejemplo conceptual: Suzuka 130R) con dos casos: iniciar ataque desde atrás → se abstiene si no hay ventaja; llegar emparejado → cede espacio y sobrevive a la curva. Para el primer prototipo usar curva equivalente de Fuji.

**Referencias:** [ForzaETH `_check_ot_sector()` y transiciones](https://github.com/ForzaETH/race_stack/blob/main/state_machine/src/state_machine_node.py), [ForzaETH transiciones](https://github.com/ForzaETH/race_stack/blob/main/state_machine/src/state_transitions.py), [TUM trajectory optimization](https://github.com/TUMFTM/global_racetrajectory_optimization).

**Aceptación:** 100 pruebas de oportunidades etiquetadas; en 100/100 casos sin ventaja o con sector incompatible no inicia ataque nominal; en casos ya rueda a rueda no pierde conciencia del rival por metadato de zona.

### M3-02 — Diferencial de ritmo ≠ diferencial instantáneo de velocidad

- [ ] Modelar `free_air_lap_pace` a partir de ritmo reciente sin tráfico, setup, neumáticos y condiciones de pista; no confundirlo con velocidad instantánea en un punto.
- [ ] Separar: `pace_advantage_seconds_per_lap` (motivo para perseguir) de `local_overtake_feasibility` (si puede completar la maniobra antes de un punto futuro).
- [ ] Calcular distancia/tiempo de alcance estimado, ventaja de salida y velocidad relativa en la futura zona; incorporar ancho necesario y posible respuesta defensiva rival.
- [ ] Descartar divisiones por velocidad relativa cero y horizontes absurdamente largos. No intentar adelantar en un punto donde la velocidad de cierre y distancia no dan para construir overlap.
- [ ] Generar al menos 3 trayectorias locales candidatas: seguir, izquierda, derecha; generar perfil de velocidad **individual por trayectoria**.

**Referencias:** [Speed Dreams `GetTakeoverOpp()` y `FilterTakeoverOffset()`](https://github.com/argos-research/speed-dreams/blob/master/src/drivers/kilo2008/kdriver.cpp), [ForzaETH estados y waypoints](https://github.com/ForzaETH/race_stack/blob/main/state_machine/src/states.py), [TUM helpers](https://github.com/TUMFTM/trajectory_planning_helpers).

**Aceptación:** 100 escenarios etiquetados con cierre real/potencial, sin intento injustificado por un simple pico de velocidad; cada trayectoria candidata tiene anchura libre, radio y velocidad propia y es descartada si es físicamente imposible.

### M3-03 — Máquina de estados con compromiso y escape

```text
FOLLOW_RACING_LINE
  -> FOLLOW_OPPONENT
  -> PREPARE_ATTACK
  -> COMMIT_ATTACK
  -> SIDE_BY_SIDE
  -> CLEAR_OPPONENT
  -> RETURN_TO_LINE
  -> FOLLOW_RACING_LINE

Cualquier estado -> RECOVERY/EMERGENCY cuando el safety supervisor veta la maniobra.
PREPARE_ATTACK -> FOLLOW_OPPONENT si desaparece la oportunidad.
COMMIT_ATTACK / SIDE_BY_SIDE -> YIELD_OR_ABORT si no hay trayecto seguro,
siempre evitando el salto inmediato a la racing line ocupada.
```

- [ ] Guardar rival objetivo, lado elegido, tiempo de compromiso, posición relativa e histéresis de entrada/salida.
- [ ] Definir criterio geométrico de finalización: longitudinalmente despejado + margen + estabilidad durante intervalo mínimo; no declarar "completado" solo por posición del centro de masa.
- [ ] Al abortar, buscar trayectoria de retorno segura. Abortar no significa girar bruscamente hacia la racing line.
- [ ] Controlar defensa legal: una maniobra de defensa permitida, sin movimiento reactivo brusco después de compromiso, manteniendo espacio cuando hay overlap.
- [ ] El objetivo táctico local tiene prioridad sobre un cambio macro ordinario una vez comprometido; seguridad/rules pueden vetar en cualquier momento.

**Código:** [ForzaETH `state_transitions.py`](https://github.com/ForzaETH/race_stack/blob/main/state_machine/src/state_transitions.py), [ForzaETH `states.py`](https://github.com/ForzaETH/race_stack/blob/main/state_machine/src/states.py), [TORCS robot tutorial ch. 5](https://torcs.sourceforge.net/api/robot_tutorial_chapter_5.html).

**Aceptación:** ≥100 maniobras de laboratorio (50 por lado), sin transiciones oscilatorias >2 cambios/s entre estados de conflicto; **0 retornos deliberados a una trayectoria ocupada**; todos los abortos con razón trazada.

### M3-04 — Predicción y safety supervisor

- [ ] Proyectar en horizonte corto (inicial 1–3 s adaptado a velocidad y zona) las trayectorias propia y rival con envolventes de incertidumbre y tamaño físico.
- [ ] Detectar colisiones previsibles por overlap de volumen espacio-tiempo; penalizar estrechez, no asumir que el rival mantendrá trayectoria ideal.
- [ ] Considerar maniobra defensiva y cambios razonables de ritmo/posición del rival; trabajar con **pequeño conjunto de hipótesis**, no un único futuro perfecto.
- [ ] Separar safety como **filtro duro** del ranking `Utility`: primero descartar candidatos inseguros, luego optimizar tiempo/posición.
- [ ] Incorporar reglas deportivas como límites de pista y espacio lateral; no equiparar "self-preservation" con renunciar a todo duelo (la IA puede correr riesgos razonables, no accidentes previsibles).
- [ ] Alerta crítica de colisión y frenado debe operar a frecuencia alta, independiente de la máquina de estados de 5–10 Hz.

**Referencias:** [CARLA CollisionStage](https://github.com/carla-simulator/carla/blob/ue5-dev/LibCarla/source/carla/trafficmanager/CollisionStage.cpp), [CARLA docs](https://carla.readthedocs.io/en/latest/adv_traffic_manager/), [ForzaETH predictive planner/state machine](https://github.com/ForzaETH/race_stack/blob/main/state_machine/src/state_transitions.py).

**Aceptación:** suite de escenarios: rival frena temprano, zigzag, overlap a entrada, salida estrecha, cambio de grip; cero contactos graves en casos evitables con estado inicial físicamente recuperable; medir falsas alarmas, latencia de respuesta y causas de abortos.

### M3-05 — Criterios de utilidad y perfiles de piloto

- [ ] Implementar `Utility` solo tras filtro seguridad/rules: beneficios de tiempo, adelantamiento, ahorro neumático, defensa, coste de maniobra; argumentos normalizados y trazables.
- [ ] Perfiles `CONSERVATIVE`, `BALANCED`, `AGGRESSIVE` mediante umbral de incentivo, tiempo de preparación, persistencia, preferencia de lado y margen extra **dentro de límites duros**.
- [ ] Evitar agresividad como multiplicador de potencia, grip o velocidad del coche.
- [ ] Aplicar histéresis: cambiar de intención cuando la utilidad alternativa supere margen significativo o el objetivo actual caduque/sea inviable.
- [ ] Log de puntuaciones para cada decisión: alternativas descartadas, razones físicas, ganador y objetivo macro influyente.

**Referencia:** [HighwayEnv IDM/MOBIL](https://github.com/Farama-Foundation/HighwayEnv/blob/main/highway_env/vehicle/behavior.py), [HighwayEnv repo](https://github.com/Farama-Foundation/HighwayEnv), [Speed Dreams oponentes](https://github.com/argos-research/speed-dreams/blob/master/src/drivers/kilo2008/opponent.cpp).

**Aceptación:** perfiles agresivos **intentan** más maniobras viables que conservadores bajo 100 oportunidades idénticas (medir intentos/100 oportunidades); no aumento aceptable de accidentes evitables; el mismo coche mantiene el mismo límite físico en todos los perfiles.

### PUERTA M3

- [ ] En 4 coches, completar al menos **50 carreras de 5 vueltas** con variedad de grid/ritmos/seeds.
- [ ] Registrar oportunidades, intentos, abandonos, maniobras completadas, contactos, fallos por sector, cambios de estado y rendimiento.
- [ ] Meta inicial: ≥70% de intentos comprometidos terminan en adelantamiento o abandono **sin contacto evitable**; **≤1 contacto evitable por 100 intentos**, umbrales sujetos a definición de severidad y revisión por replay (sin contabilizar un roce inevitable como igual a una embestida).
- [ ] Mantener el presupuesto de P0/M2; si la predicción de trayectorias eleva p95, reducir número de candidatos/frecuencia, no ocultar la latencia.

**Entrega M3:** batallas locales emergentes sin instrucciones específicas de la IA global.

## 8. M4 — IA macro imparcial: estrategia, objetivos y boxes

**Dependencia:** M2 aprobado para integrar objetivos; recomendable M3 para evaluar su impacto en luchas. La estrategia **no** decide pedal, dirección, agresividad instantánea ni qué piloto "merece" ganar. Todos siguen las mismas reglas de decisión estratégica pero con datos y estado propio.

### M4-01 — Director estratégico de baja frecuencia

- [ ] Publicar un snapshot **consistente** de carrera (posición, gaps, neumáticos, desgaste, combustible, ritmo libre, daños, penalizaciones, ventanas de boxes, clima si existe).
- [ ] Construir estimador de tiempo de carrera para varias estrategias (continuar, parar ahora, parar después, conservar, empujar). Comenzar determinista; posterior Monte Carlo solo si aporta valor.
- [ ] Ejecutar cada 1–5 segundos de simulación según coste y activar recálculo ante eventos significativos (pinchazo, abandono, Safety Car, cambio de grip, pit stop rival, degradación inesperada).
- [ ] Evitar instrucciones triviales oscilantes; comparar mejora frente a estrategia vigente, coste de cambio y ventana de compromiso.
- [ ] Ser imparcial en la fórmula: decisiones específicas por coche son producto de datos/objetivos individuales, **no** favorecimiento manual del jugador o piloto.
- [ ] Guardar justificación cuantitativa: predicción de finish time, ahorro esperado, degradación, tráfico previsto, duración pit lane y nivel de incertidumbre.

**Referencias:** [Speed Dreams `strategy.cpp` heurística de paradas](https://github.com/argos-research/speed-dreams/blob/master/src/drivers/kilo2008/strategy.cpp), [Pitwall — proyecto de estrategia de carrera](https://github.com/Hellblazer704/pitwall), [CARLA Traffic Manager — sincronización/cache](https://github.com/carla-simulator/carla/blob/ue5-dev/Docs/adv_traffic_manager.md).

**Advertencia sobre F1 2030:** el concepto actual **no utiliza repostaje durante la carrera**. No portar literalmente la estrategia histórica de combustible/repostaje de Speed Dreams: adaptar optimización a **neumáticos, tráfico, pérdida en pit lane, daños, condiciones y reglas**. El combustible influye en masa/ritmo/consumo, pero **no habilitar refueling en boxes** por haber copiado un algoritmo antiguo.

### M4-02 — Contrato de objetivos y feedback

- [ ] Enviar `DriverObjective` firmado por `objective_identifier`, `issued_at_tick`, `expires_at_tick`, `reason`, parámetros y versión; no asignar overrides de throttle, velocidad física, ni steering.
- [ ] Implementar `ObjectiveFeedback`: `ACCEPTED`, `DEFERRED`, `REJECTED`, `COMPLETED` con razón (`PUNCTURE`, `TRAFFIC`, `INSUFFICIENT_GRIP`, `SAFETY`, `PIT_ENTRY_UNSAFE`, etc.).
- [ ] La IA local prioriza self-preservation y reglas; objetivos estratégicos ocupan **capa inferior de utilidad** y jamás suspenden control inmediato.
- [ ] La IA local inicia adelantamientos por oportunidad aunque no exista orden `ATTACK_TARGET`; al contrario, puede no adelantar aunque exista objetivo de ataque si las condiciones no lo permiten.
- [ ] Configurar timeout; no exigir respuesta síncrona del director para frenar o entrar en recuperación. Prioridad de objetivos globales no crea prioridades arbitrarias sobre seguridad.

**Referencia interna:** [`ai_stub.rs` actual (reemplazo semántico necesario)](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/formula90-core/src/modules/ai_stub.rs).<br>
**Referencia externa de separación decisiones/manipulación:** [ForzaETH state machine vs controllers](https://github.com/ForzaETH/race_stack/tree/main/state_machine), [CARLA control loop](https://carla.readthedocs.io/en/latest/adv_traffic_manager/).

**Aceptación:** ≥20 casos automatizados con objetivo válido/expirado/sustituido/rechazado, feedback trazado; ningún mensaje global llega directamente a motor físico como teletransporte, multiplicador o input obligatorio.

### M4-03 — Modelos de degradación, ritmo y oportunidades de boxes

- [ ] Estimar degradación por compuesto, temperatura, stint, carga y estilo; distinguir tiempo por vuelta propio y tiempo con tráfico.
- [ ] Calcular costo completo de pit (entrada, limitador, parada, salida, reincorporación), disponibilidad de puesto, neumáticos disponibles y ventanas de cumplimiento reglamentario.
- [ ] Comparar undercut/overcut, neumáticos frescos vs neumáticos usados y riesgo de volver detrás de tráfico.
- [ ] No exigir que piloto en problemas cumpla `FASTEST_LAP`; su controlador puede aceptar/deferir según neumático y espacio disponible.
- [ ] Soportar parada de emergencia **iniciada localmente** por pinchazo/daño grave sin esperar nuevo ciclo macro.
- [ ] Registrar error de predicción estrategia vs resultado real para calibración posterior.

**Referencias:** [Pitwall](https://github.com/Hellblazer704/pitwall), [Speed Dreams pit](https://github.com/argos-research/speed-dreams/blob/master/src/drivers/kilo2008/pit.cpp), [Speed Dreams strategy](https://github.com/argos-research/speed-dreams/blob/master/src/drivers/kilo2008/strategy.cpp), [Formula 2030 pit stop controller actual](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/scripts/runtime/pit_stop_controller.gd), [reglas pit](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/scripts/runtime/pit_stop_rules.gd).

**Aceptación:** 20 escenarios reproducibles con desgaste/ventana alterados, selección de parada explicable; ≥95% de instrucciones de boxes no expiran antes de permitir entrada con margen de frenado; emergency pit local no bloqueado por el director. Cero repostajes accidentalmente habilitados.

### M4-04 — Fairness, información y credibilidad

- [ ] Estratega usa datos de carrera disponibles coherentes (tiempos, posiciones, neumáticos propios, estimaciones de otros); **no** acceso omnisciente al desgaste exacto de rivales si el jugador no lo tendría.
- [ ] Introducir incertidumbre calibrable en pronósticos estratégicos; compartición de cómputo no implica percepción perfecta de cada IA local.
- [ ] Las decisiones estratégicas registran la misma función objetivo y pesos para todos, salvo preferencias explícitas de piloto/equipo documentadas.
- [ ] Separar estrategias de equipo legítimas (cooperación explícita) del favoritismo del director central.

### PUERTA M4

- [ ] Test de objetivos: `PUSH_PACE` aceptado con pista libre; diferido con tráfico; rechazado con pinchazo; en los tres casos el vehículo mantiene control correcto.
- [ ] Test de pit: entrada, limitador, servicio, salida y reincorporación con 1 coche y luego parrilla de 4 sin colisión por trayectoria imposible.
- [ ] En al menos 30 escenarios sembrados, planificador evita planes estratégicos de mayor tiempo previsto si las alternativas tienen mismos supuestos; log de justificación completo.
- [ ] **Ningún** byte de estrategia se interpreta como throttle/steering override ni altera la física de un rival.

**Entrega M4:** IA de carrera con estrategia de muro de boxes y pilotos autónomos, con autoridad claramente separada.

## 9. M5 — Robustez, realismo y escalabilidad

### M5-01 — Situaciones anómalas y recuperación

- [ ] Neumático pinchado, frenada desigual, trompo, coche atravesado, motor apagado, fallo de cambio, salida de pista y lluvia futura; identificar qué estados físicos existen realmente antes de prometer su simulación.
- [ ] Prioridades: sobrevivir/evitar choque, contener vehículo, recuperar pista con seguridad, retomar ritmo, informar al macro.
- [ ] Recuperación sin reset invisibles salvo reglas de juego explícitas; colisiones no desaparecen para favorecer IA.
- [ ] Dirty air, slipstream, tyre wear y carga aerodinámica alteran la **factibilidad de maniobras**, no se parchean con velocidad arbitraria.

**Referencias:** [VDrift recuperación/raycast de obstáculo](https://github.com/VDrift/vdrift/blob/master/src/ai/ai_car_experimental.cpp), [CARLA hazards](https://github.com/carla-simulator/carla/blob/ue5-dev/LibCarla/source/carla/trafficmanager/CollisionStage.cpp), [ForzaETH `FTGONLY`](https://github.com/ForzaETH/race_stack/blob/main/state_machine/src/states.py).

### M5-02 — Validación de 20 vehículos (posterior)

- [ ] Escalonar 4 → 8 → 12 → 20 con mismo escenario, presupuesto y pruebas de colisiones; comparar CPU por módulo y memoria por entidad.
- [ ] Predicción compartida de rivales con índice espacial/Frenet y sólo coches realmente relevantes en horizonte.
- [ ] Aplicar escalado de frecuencia **solo a planificación táctica cara/animaciones/audio distante**, no a contactos físicos ni reacción de seguridad cuando hay proximidad.
- [ ] Benchmark de salida en parrilla, sectores densos, pit lane, lluvia futura, Safety Car futuro y retransmisión/cámaras.
- [ ] Preservar configuración `AI_ENGINE_ONLY` para voces lejanas, sin crear una instancia pesada de mezcla completa por IA.

**Aceptación preliminar:** 20 coches física estable, 10 vueltas por seed ×3 seeds, sin NaN/Inf ni pérdida de autoría de inputs; FPS/latencia publicados. **No** extrapolar resultados de 4 coches linealmente sin medición.

### M5-03 — Herramientas de ingeniería y depuración

- [ ] Overlay: racing line, target waypoints, curva de velocidad, s/d, trayectorias candidatas, TTC, occupancy, objetivo macro, estado táctico, veto safety, razones de descarte.
- [ ] Replays de `snapshot + input + decisión + seed + git SHA + config digest`, exportados por modo test; guardar video solo opcionalmente.
- [ ] Tablero de KPIs por build/commit con tendencia p50/p95/p99, violaciones físicas, choques evitables, adelantamientos y delta de vuelta.
- [ ] Herramienta de "freeze/replay decision" para poder reproducir un incidente sin ejecutar la carrera completa.
- [ ] Pruebas CI headless al menos para pure Rust y pruebas de integración Godot compatibles con entorno.

## 10. Matriz de hitos y criterios medibles (resumen ejecutable)

| Hito | Precondiciones | Prueba principal | Umbral de aprobación | Artefactos |
|---|---|---|---|---|
| **P0** Coste y física | BUILD/HEAD coherentes | 1/2/4 vehículos, 10k ticks | baseline completo; perfil aprobado; sin NaN/Inf | JSON métricas, informe FULL/LITE |
| **M1** 1 IA sola | P0 esencial | 3 seeds × 2 runs × 10 laps | vueltas completas, sin salidas, p95 error ≤1 m | escena, telemetría, gráficos |
| **M2** 4 coches follow | M1 + P0 | 5 seeds × 10 laps ×4 | sin embestidas nominales; presupuesto CPU/audio | parrilla, logs, render/headless |
| **M3** Adelantar | M2 | 50 carreras ×5 laps + 100 maniobras | frecuencia de incidentes y estados bajo umbral | replays con causas |
| **M4** Estrategia macro | M2, ideal M3 | 30 escenarios + 20 contratos | rechazo seguro, boxes coherentes, sin rubber-band | predicciones y feedback |
| **M5** Escalado | M3/M4 | 4/8/12/20 vehículos | resultados medidos y reproducibles | reportes de escalabilidad |

### Definiciones de métricas (no aceptarlas ambiguas)

- `lap_completion_rate = vueltas válidas sin reset / vueltas solicitadas`.
- `lateral_error_p95_metres = percentil95(abs(distancia del centro del coche a racing line))`, solo durante seguimiento normal; **no** calcular sobre un adelantamiento legítimo.
- `offtrack_events`: centro + semiancho/sobreanchos reales del vehículo fuera de límites válidos; registrar duración y distancia máxima. Definir tolerancia de piano/legal según reglas del circuito.
- `avoidable_collision`: contacto donde al menos una maniobra segura era físicamente posible con observación y tiempo de reacción razonables. Evaluar con replay y contrafactual, no etiqueta automática solo por impulso.
- `overtaking_opportunity`: pareja y sector que cumplen puerta de ritmo y ventana física; identificador único para no contar 50 oportunidades por 50 ticks de un solo ataque.
- `overtake_attempt`: transición confirmada a `COMMIT_ATTACK`; registrar lado/rival/sector/seed.
- `overtake_success`: se supera al rival longitudinalmente con distancia segura y se retiene posición tras zona de verificación.
- `contact_rate_per_100_attempts = 100 * contactos evitables / intentos`.
- `decision_chatter = nº transiciones de estado por segundo` en ventana estable; excluir emergencias legítimas.
- `physical_tick_p95_ms`: p95 de tiempo CPU de una actualización autoritativa de **todo el mundo** en ventana fija tras warmup.
- `ai_decision_p95_ms`: p95 total de decidir para las 4 IA por ejecución; publicar además promedio amortizado por frame.
- `audio_underruns`: bloques que no pudieron entregar muestras a tiempo; objetivo cero durante validación estable.
- `determinism_hash`: hash de serie ordenada de inputs/estados redondeados según especificación, seed, HEAD y dt.
- `pace_coefficient_of_variation`: desviación estándar / media de vueltas limpias comparables, sin tráfico ni pit.

### Reporte mínimo `benchmark_results.json` (esquema ilustrativo)

```json
{
  "project": "formula_2030_race_intelligence",
  "git_commit": "REPLACE_WITH_REAL_SHA",
  "build_source": "REPLACE_WITH_SOURCE_IDENTIFIER",
  "scenario": "four_drivers_following",
  "seed": 42,
  "physics_mode": "FULL_OR_AI_LITE",
  "physics_frequency_hertz": 120,
  "vehicle_count": 4,
  "laps_per_vehicle": 10,
  "hardware_profile": "REPLACE_WITH_TEST_MACHINE_PROFILE",
  "physics_tick_time_milliseconds": {
    "median": null,
    "p95": null,
    "p99": null
  },
  "local_decision_time_milliseconds_p95": null,
  "render_frame_time_milliseconds_p95": null,
  "audio_active_ai_engine_voices_max": null,
  "audio_underruns": null,
  "lap_completion_rate": null,
  "lateral_error_p95_metres": null,
  "avoidable_collisions": null,
  "race_decision_determinism_hash": null
}
```

**Métrica no disponible = `null`, jamás cero inventado.** Cada reporte adjunta log de herramientas/comando real, versión Rust/Godot, orden de actualización, config de sensores y perfil audio. Construir comparaciones entre commits sin mezclar hardware o escena.

## 11. Plan de entrega en PRs atómicos (orden recomendado para un agente Codex)

| PR | Prioridad | Alcance estricto y archivos objetivo | Bloqueo/validación |
|---|---|---|---|
| **PR-01** | P0 | Instrumentación de `coupled_vehicle_world.rs`, `physical_collision_world.rs`, puente y runner de benchmark; baseline inmutable | Reporte 1/2/4 real, commits/BUILD documentados |
| **PR-02** | P0 | Audio engine-only desde `audio_worker.rs` + policy/config del mixer; medir voces | Cero underruns, máximo 2 voces IA activas por defecto |
| **PR-03** | P0 | Solo si hace falta: perfil FULL/LITE en dominio `vehicle-physics-engine`, desactivación de rig legacy redundante, paridad física | Misma carga física sin contactos perdidos; no cambios invisibles del player |
| **PR-04** | M1 | Nuevo `race-intelligence-engine` (si arquitectura confirmada), lector/proyector Fuji, tests geométricos | `s/d` válidos y vueltas con wrap correcto |
| **PR-05** | M1 | Perfil de velocidad, dirección, freno, acelerador y marchas Rust; integración `VehicleInput` | Una vuelta completa sin intervención, plots |
| **PR-06** | M1 | Escena standalone, telemetría, runner de 10 laps x 3 seeds x 2 runs | Puerta M1 con evidencia |
| **PR-07** | M2 | Grid 4, spawn, IDs, única autoridad de inputs, snapshots consistentes | 4 coches con registros independientes y FPS medidos |
| **PR-08** | M2 | Opponent tracking, TTC, follow, histéresis y tests de seguridad | Puerta M2 5 seeds × 10 laps × 4 |
| **PR-09** | M3 | Sectores, candidatos de trayectoria, gate diferencial ritmo + zona | Suite de ≥100 oportunidades |
| **PR-10** | M3 | HFSM de racecraft, conducción emparejada, defensa/abort | Replays y puerta M3 |
| **PR-11** | M4 | Objetivos serializados, feedback, caducidad y puente macro/local | 20 casos de contrato sin input global directo |
| **PR-12** | M4 | Predicción stint/neumáticos, pits, decisiones de estrategia | Puerta M4 y 30 escenarios |
| **PR-13** | M5 | Escalabilidad, nuevas pruebas de fallo, QA y observabilidad | comparativas 4/8/12/20 |

**Política por PR:** cada rama debe estar basada en HEAD actual inspeccionado; no reusar binarios ignorados de otra rama; pruebas focalizadas + regresión de subsistemas afectados + chequeo BUILD/HEAD; diffs/commits atómicos sin incluir archivos externos o preexistentes. Respetar `AGENTS.md`, incluyendo convención de nombres y prohibición de agregar comentarios de código nuevos en este proyecto.

### Primer sprint exacto: lo que debe hacer el agente mañana

**Nota de seguimiento:** este listado conserva el plan inicial. Para continuar el trabajo ya iniciado, seguir el orden de reanudación de 0.1 y los bloqueos P0-06/P0-07; no reiniciar la implementación ni declarar aprobadas sus puertas.

1. Inspeccionar `game/AGENTS.md`, `game/crates/AGENTS.md`, `game/crates/vehicle-physics-engine/AGENTS.md`, `native/AGENTS.md` y `game/sounds/AGENTS.md`; dejar constancia de branch, HEAD, status y BUILD.
2. Confirmar qué escena real de F1 2030 utiliza el `PhysicalVehicleWorldController`: no asumir que el prototipo legacy con 12 RayCast3D es el camino en uso. Verificar escena/entidades/snapshots/ownership.
3. Leer función de consultas de `coupled_vehicle_world.rs`, contar suelo/aero/contactos. Leer tests físicos existentes y probar 4 instancias bajo mismo mundo.
4. Ejecutar baseline con el runner existente [`game/tests/perf_capture_f1_2030_windowed.gd`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/tests/perf_capture_f1_2030_windowed.gd) y extender con contadores y escena de 4 entidades. NO asumir p95 actual.
5. Auditar dos mundos diferentes: [`game/crates/game-sim/src/world.rs`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/game-sim/src/world.rs) expone una simulación lógica que usa `flat_ground_samples`; **no confundirla con el mundo de colisión de circuito real `CoupledVehicleWorld`**. Identificar el host autoritativo de la escena de prueba antes de conectar IA. La arquitectura final no debe mover pilotos a un mundo de suelo plano para conseguir FPS artificiales.
6. Revisar el banco canónico [`game/sounds/banks/v10-v2-bank`](https://github.com/Anthony38zrgo-2/formula-90/tree/main-clean/game/sounds/banks/v10-v2-bank) y [`game/sounds/AGENTS.md`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/sounds/AGENTS.md): no crear banco runtime en `game/audio/` ni editar WAVs derivados manualmente.
7. Publicar reporte con **GO/NO-GO para 4 vehículos** antes de decidir perfiles LITE. Si no pasa, identificar coste dominante y PR concreto con ahorro esperado.
8. Solo después comenzar `TrackPathSample`, controlador `FOLLOW_RACING_LINE`, `VehicleInput` y escena de una IA; marcar cada test de M1 con resultado medido.

## 12. Referencias de código open source por responsabilidad

### 12.1 Tabla de adopción técnica

| Apartado de IA | Código real de referencia | Qué estudiar / llevarse | Advertencia |
|---|---|---|---|
| Línea óptima de pista | [TUM global trajectory](https://github.com/TUMFTM/global_racetrajectory_optimization/blob/master/main_globaltraj.py); [helpers](https://github.com/TUMFTM/trajectory_planning_helpers) | Línea mínima curvatura/tiempo, límite lateral, generación offline | Optimizar **fuera** del tick; no ejecutar solver global cada frame |
| Seguimiento de línea | [ForzaETH `controller/pp`](https://github.com/ForzaETH/race_stack/tree/main/controller/pp); [VDrift AI Standard](https://github.com/VDrift/vdrift/blob/master/src/ai/ai_car_standard.cpp) | Pure Pursuit/lookahead, control básico y corrección | Necesita adaptar a steering, grip y velocidad F1 propia |
| Perfil de velocidad/freno | [VDrift AI Experimental](https://github.com/VDrift/vdrift/blob/master/src/ai/ai_car_experimental.cpp) | Curvatura, velocidad máxima, distancia de frenado | Fórmulas calibradas a otros coches, no copiar coeficientes |
| Oponentes y espacios | [Speed Dreams `opponent.cpp`](https://github.com/argos-research/speed-dreams/blob/master/src/drivers/kilo2008/opponent.cpp) | Estado frontal/trasero/lateral, wrapping de vuelta, márgenes | Heurísticas antiguas; mejorar predicción temporal |
| Follow y adelantamiento | [Speed Dreams `kdriver.cpp`](https://github.com/argos-research/speed-dreams/blob/master/src/drivers/kilo2008/kdriver.cpp); [TORCS tutorial capítulo 5](https://torcs.sourceforge.net/api/robot_tutorial_chapter_5.html) | `GetTakeoverOpp`, elección de lateral, estados persistentes | No copiar offsets arbitrarios sin asegurar trayectoria viable |
| Máquina de estados | [ForzaETH `state_transitions.py`](https://github.com/ForzaETH/race_stack/blob/main/state_machine/src/state_transitions.py); [`states.py`](https://github.com/ForzaETH/race_stack/blob/main/state_machine/src/states.py) | `GB_TRACK`, `TRAILING`, `OVERTAKE`, zonas y trayectoria libre | Añadir fases de preparación/compromiso/overlap específicas F1 |
| Zonas de adelantamiento | [ForzaETH `state_machine_node.py`](https://github.com/ForzaETH/race_stack/blob/main/state_machine/src/state_machine_node.py) | `_check_ot_sector`, `_check_ofree`, caducidad de spline | Sector como gate **de inicio**, no como permiso de ignorar rival en plena batalla |
| Selección de comportamiento | [HighwayEnv `behavior.py`](https://github.com/Farama-Foundation/HighwayEnv/blob/main/highway_env/vehicle/behavior.py) | IDM/MOBIL: separar seguridad e incentivo | Autopista no es circuito; no copiar carriles/rules literalmente |
| Trajectorias/colisiones predictivas | [CARLA `CollisionStage.cpp`](https://github.com/carla-simulator/carla/blob/ue5-dev/LibCarla/source/carla/trafficmanager/CollisionStage.cpp) | Bounding boxes extendidas, pares de conflicto, posibles colisiones | Evitar copia de tráfico vial: horizontes y grip F1 diferentes |
| Snapshot y scheduling central | [CARLA `TrafficManagerLocal.cpp`](https://github.com/carla-simulator/carla/blob/ue5-dev/LibCarla/source/carla/trafficmanager/TrafficManagerLocal.cpp); [docs](https://carla.readthedocs.io/en/latest/adv_traffic_manager/) | Cache de estado común, barreras por etapa, aplicación en batch | Compartir cálculo ≠ centralizar autonomía decisional |
| Estrategia y pit | [Speed Dreams `strategy.cpp`](https://github.com/argos-research/speed-dreams/blob/master/src/drivers/kilo2008/strategy.cpp); [`pit.cpp`](https://github.com/argos-research/speed-dreams/blob/master/src/drivers/kilo2008/pit.cpp) | Coste de parada, mantenimiento, decisiones recurrentes | Su estrategia de **repostaje** no aplica a este F1 2030 |
| Estrategia avanzada | [Pitwall](https://github.com/Hellblazer704/pitwall) | Modelos de carrera/degradación, evaluación de escenarios | Verificar supuestos, datos, licencia y completitud antes de portar |
| Física autoritativa | [Formula-90 `coupled_vehicle_world.rs`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/vehicle-physics-engine/src/coupled_vehicle_world.rs) | Un solo mundo, raycast principal/rueda, contacto, inputs y snapshots | **Es el punto de integración**, no un repositorio que sustituir |
| Audio motor-only IA | [Formula-90 audio worker](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/formula90-core/src/audio_worker.rs) | Mixer y worker desacoplado; limitar voces/updates | No degradar audio del jugador; no duplicar mezcladores por entidad |
| Bench y QA | [Formula-90 perf windowed](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/tests/perf_capture_f1_2030_windowed.gd); [tests del mundo físico](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/vehicle-physics-engine/tests/coupled_vehicle_world_test.rs) | percentiles y escenarios Rust/Godot reproducibles | Mantener hardware/build/escenas constantes |

### 12.2 Licencias y trazabilidad de terceros

- [ ] Para **cada** referencia externa, guardar URL de repo, SHA/commit, archivo, función revisada y licencia correspondiente en `docs/race-intelligence/reference-attribution.md`.
- [ ] Si solo se adopta una **idea/algoritmo**, documentar la inspiración y escribir implementación original adaptada a física Rust.
- [ ] **No copiar código** de Speed Dreams, VDrift, TORCS u otros proyectos GPL/LGPL sin auditoría legal del efecto de su licencia sobre la distribución del simulador. Lo mismo aplica a fragmentos de TUM bajo LGPL o dependencias. Un enlace open source **no equivale a permiso irrestricto de relicenciar**.
- [ ] Si se importa código permitido, preservar avisos y atribuciones exigidos y registrar en `THIRD_PARTY.md`; no confundir "referencia de código" con componente realmente integrado.
- [ ] No usar código ni assets de Codemasters/F1 comerciales: pueden inspirar **comportamiento observado**, pero no son referencia de implementación abierta.

## 13. Batería de escenarios automatizados

| ID | Nombre | Nº vehículos | Estímulo | Afirmación obligatoria |
|---|---|---:|---|---|
| `LINE-001` | Recta + curva | 1 | Fuji limpio | throttle, brake y steer válidos |
| `LINE-002` | Cierre de vuelta | 1 | índice cercano a último waypoint | `s` continúa sin salto |
| `LINE-003` | Baja velocidad | 1 | spawn detenido | arranca sin oscilación/error |
| `LINE-004` | Perturbación lateral | 1 | spawn +1 m offset | recupera racing line físicamente |
| `LINE-005` | Curva estrecha | 1 | target alto inicial | anticipa frenada; no sale |
| `LINE-006` | 10 vueltas | 1 | 3 seeds ×2 runs | puerta M1 |
| `GRID-001` | Spawns | 4 | parrilla alternada | sin overlaps al iniciar |
| `GRID-002` | Follow lento | 4 | coche delante −10% target pace | followers frenan, no embisten |
| `GRID-003` | Frenada inesperada | 2–4 | líder frena antes | mitigación dentro de física posible |
| `GRID-004` | Meta/vuelta solapada | 4 | distancias cerca de 0/s_fin | orden de rivals correcto |
| `GRID-005` | Duración y rendimiento | 4 | 5 seeds ×10 vueltas | puerta M2 y P0 |
| `PASS-001` | Sin ventaja | 2 | igual ritmo, curva rápida | no inicia ataque |
| `PASS-002` | Ventaja y zona apta | 2 | ventaja en recta/frenada | evalúa y ejecuta si viable |
| `PASS-003` | Rueda a rueda en zona desfavorable | 2 | overlap previo | no ignora al rival |
| `PASS-004` | Attack abort | 2 | rival cierra ventana legalmente | retorno seguro |
| `PASS-005` | Curva exterior | 2 | candidato exterior | perfil de velocidad propio |
| `PASS-006` | Perfil de piloto | 2 | mismos datos, agresividad distinta | intención distinta, física idéntica |
| `PIT-001` | Objetivo de empujar | 1 | pista libre | objetivo aceptado |
| `PIT-002` | Push con tráfico | 4 | bloqueado por rival | diferido sin colisión |
| `PIT-003` | Push con pinchazo | 1 | daño grave | rechazo por self-preservation |
| `PIT-004` | Stop estratégico | 4 | undercut probable | decisión justificada |
| `PIT-005` | Stop urgente | 1 | pinchazo inmediato | ejecución local sin esperar macro |
| `PERF-001` | FULL vs AI_LITE | 1/4 | inputs idénticos | diferencias dentro de tolerancia |
| `PERF-002` | Audio culling | 4 | cámara se aleja/acerca | ≤2 voces IA por defecto, sin clicks |
| `REPLAY-001` | Determinismo | 4 | mismo seed, 3 runs | hash/precisión dentro contrato |

## 14. Registro de riesgos y decisiones abiertas

| Riesgo | Señal de alarma | Mitigación | Momento de resolver |
|---|---|---|---|
| Física real demasiado costosa | p95 tick supera presupuesto | perfilar contactos/solver/aero antes de bajar raycasts; LITE justificado | **P0, bloqueante** |
| Amortiguación explícita y contactos inestables a 120 Hz | oscilación autosostenida, topes, vuelco o error de solapamiento | corrección implícita candidata + reproducción de contactos; no compensar con grip o rebote arbitrarios | **P0-06, bloqueante actual** |
| Replay falla antes de simular | JSON `36.0` rechazado para `u32` | reparar contrato/adaptador numérico y repetir inputs idénticos | **P0-07** |
| Confundir resultado histórico con validación actual | una vuelta limitada se presenta como M1 o como aprobación del último DLL | resultados por fuente/configuración; mantener puerta de diez vueltas abierta | **P0/M1** |
| Worlds duplicados (`game-sim` vs coupled world) | telemetría plana distinta de colisión real | declarar propietario físico por escena; eliminar avance doble | **P0** |
| `TriRaycastSample` simplificado pierde contacto de borde | rebotes/inestabilidad en pianos | pruebas de piano/bache, fallback de normal/surface o consultas selectivas | **P0** |
| Raycasts de Godot y Rust redundantes | conteo superior al previsto | medir y desactivar solo ruta inactiva tras auditar consumidores | **P0** |
| Sonido del motor por coche sigue siendo caro | múltiples capas DSP y saturación | voces lógicas limitadas, budget CPU, mixer compartido | **P0** |
| Coche no logra una vuelta | error de signo/axes, freno demasiado tarde | inspección de ejes/waypoint, overlay, perfil conservador | **M1** |
| Cuatro coches embisten | no hay follow/TTC o se actualizan en orden desigual | snapshot común, guardas locales, histéresis, distancias reales | **M2** |
| IA nunca adelanta | thresholds demasiado estrictos | medir oportunidades verdaderas, calibrar incentivo manteniendo safety | **M3** |
| IA adelanta en curva absurda | mirar solo delta instantáneo | exigir ventaja de ritmo + sector apto + maniobra físicamente viable | **M3** |
| IA abandona maniobra cuando se empareja | HFSM sin memoria de compromiso | estados `SIDE_BY_SIDE`, `HOLD`, validación de retorno seguro | **M3** |
| Macro administra todas las peleas | órdenes obligatorias por rival/curva | macro = incentivos; oportunidad y ejecución = local | **M4** |
| Copiar estrategia de repostaje | pit stops irrealistas | modelo neumáticos/tiempo de pits, **sin repostaje** | **M4** |
| Determinismo roto por paralelismo | distintas colisiones por orden de tick | snapshot inmutable, staging, orden estable, seeds documentadas | **P0 en adelante** |
| Licencias incompatibles | copiar módulos GPL sin revisar | inspiración/reimplementación original, revisión jurídica, atribución | **cada PR** |

## 15. Definition of Done general

Una casilla `[x]` solo puede marcarse cuando existe **evidencia enlazada** al commit/PR y la prueba pasó. La evidencia contiene:

- Identificadores de source/build y estado de trabajo verificados.
- Cambios mínimos con alcance aprobado; tests ejecutados y resultado, no solo un commit.
- Escenario reproducible (config, seed, circuito, número de vehículos, dt, perfil de audio/física).
- KPIs medidos y comparación con baseline de hardware equivalente.
- Regresión del modo jugador y mundo físico existente, incluida audio del jugador.
- Revisión explícita de seguridad física, paridad FULL/LITE si procede y trazabilidad de código externo.
- Para puertas M1/M2/M3/M4: logs, resumen, replay/captura breve, resultados JSON/CSV y análisis de cualquier fracaso.

**Regla de avance:** jamás iniciar adelantamientos para ocultar falta de estabilidad de seguimiento; jamás bajar complejidad física sin medición; jamás usar instrucciones macro para ocultar incompetencia de control local. Si no pasa una puerta, mantenerla abierta con evidencia y crear ticket específico, no declararla terminada.

---

### Resultado esperado al terminar cada entrega

**P0:** cuatro coches físicamente viables en presupuesto de prueba y audio IA reducido, con pruebas de rendimiento honestas.<br>
**M1:** un coche sigue solo la línea de Fuji, frena y acelera correctamente, completa vueltas sostenidas.<br>
**M2:** cuatro monoplazas forman una pequeña parrilla y circulan de manera creíble, sin adelantarse aún.<br>
**M3:** cuatro pilotos compiten, se adelantan cuando hay ventaja y sitio, y pueden abortar con seguridad.<br>
**M4:** esos mismos pilotos reciben objetivos estratégicos del muro, pero deciden autónomamente y pueden rechazarlos.<br>
**M5:** arquitectura preparada para ampliar la parrilla con pruebas de estabilidad, rendimiento y carreras largas.
