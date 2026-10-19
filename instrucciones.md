# Instrucciones — estado y pendientes del mundo físico acoplado (`PhysicalWorldPackage`)

Fecha de actualización: 2026-10-19 (tras implementar los items 17 y 28–35 como candidatos).
Rama: `main-clean`.
HEAD de partida: `29d9fc521f28814a7b35e896201971b50614ba18`.
`game/BUILD_SOURCE`: `ce0f153c70c6b888964bea428ded5c300c5fe5ab` — **desactualizado**; el launcher rechaza BUILD≠HEAD hasta el rebuild.
Árbol: el commit atómico de los items 17 y 28–35 se registra en esta continuación; no se ha hecho rebuild. Los cambios ajenos de audio, DLLs, `game/BUILD_SOURCE` y `recorded_engine_profile.rs` se preservan sin incluir.

**Decisión vigente:** el destino es el mundo físico acoplado (`scripts/run_f1_94.ps1 -PhysicalWorldPackage`), opt-in hasta cerrar los gates y obtener aprobación humana explícita. El modo `RigidBody3D` de Godot + backends simplificados queda como referencia A/B y diagnóstico.

---

## 0. Resumen ejecutivo

### 0.1 Completado candidato (estado `implemented_awaiting_review`, sin aprobación humana)

| Item | Contenido | Evidencia |
|---|---|---|
| 01–06, 09 | Procedencia, referencia, contratos de masa/colisión, presupuestos, inventario y combustible | `docs/coupled_vehicle_foundation_*`, `scratch/coupled_vehicle_*` |
| 36–37 | Interfaz nativa versionada y paso único de mundo (candidato) | `docs/coupled_vehicle_godot_integration.md`, `scratch/coupled_vehicle_godot_runtime_rebuild/` |
| 17 | Recuperación de cargas de articulación y montaje desde la aceleración acoplada | `scratch/m1_item17_joint_loads_20261019/`, 5 tests |
| 28 | Manifolds persistentes con identidad de feature, anclajes, edad, warm start, reducción e invalidación | `scratch/m2_item28_persistent_manifolds_20261019/`, 10 tests |
| 29 | Impulsos normales simultáneos con LCP de conjunto activo sobre la matriz generalizada | `scratch/m2_item29_simultaneous_contact_solver_20261019/` |
| 30 | Fricción tangencial 2×2, cono, stick/slip, materiales por superficie y warm start tangencial | `scratch/m2_item30_tangential_friction_20261019/` |
| 31 | CCD no lineal con tiempo/forma/entidad, subdivisiones, fallos geométricos y presupuesto de eventos | `scratch/m2_item31_32_ccd_classification_20261019/` |
| 32 | Propietario único de respuesta: tread, impacto rígido y tope interno; diagnósticos de duplicados | `scratch/m2_item31_32_ccd_classification_20261019/` |
| 33 | Contacto vehículo-vehículo en un paso, matriz normal por identidad de entidad, islas y orden determinista | `scratch/m2_item33_multi_vehicle_contact_20261019/`, 2 tests |
| 34 | Objetos dinámicos Rust (conos y barreras), identidad y paso compartidos, intercambio de momento y ciclo de vida; vehículos legacy rechazados | `scratch/m2_item34_dynamic_world_objects_20261019/`, 7 tests |
| 35 | Reposo estable: manifolds/warm start de reposo, fricción estática con corrección tangencial, sleep/wake de objetos y validación de costuras | `scratch/m2_item35_rest_stability_20261019/`, 5 tests |

Los items anteriores siguen **pendientes de revisión humana**; “completado candidato” no significa aprobado ni activado.

### 0.2 Pendientes reales

- **M1 no está cerrado:** 07, 08, 11–16, 18–20 siguen en `implementing`; el gate de derivadas, envolventes geométricas, energía/momento y el informe de cargas requiere revisión.
- **M2 casi cerrado:** 24–27 siguen en `implementing`; 28–35 están en revisión.
- **M3–M7:** 22/46, 21/38/41/42, 39/40, 44/45, 43/47–50 siguen pendientes o en implementación.
- **Build:** `BUILD_SOURCE` no coincide con HEAD; no hay runtime válido hasta rebuild.
- **Commit:** los cambios de física de esta continuación no están commiteados ni revisados.

### 0.3 Verificación actual

`cargo test -p vehicle_physics_engine --no-fail-fast -- --test-threads=1`:

- `coupled_vehicle_world_test`: 28/28.
- `coupled_vehicle_load_report_test`: 5/5.
- `coupled_contact_manifold_test`: 10/10.
- `coupled_contact_solver_test`: 6/6.
- `coupled_continuous_collision_test`: 2/2.
- `coupled_multi_vehicle_test`: 2/2.
- `coupled_dynamic_object_test`: 7/7.
- `coupled_rest_stability_test`: 5/5.
- Fallos preexistentes: 14 (ver hallazgo 9). Ningún fallo nuevo.
- `cargo check -p vehicle_physics_engine --lib`: limpio.

Los comandos concretos y logs están en los directorios `scratch/m1_item17_*` y `scratch/m2_item*` citados en la tabla de §0.1. No se ha ejecutado el launcher canónico: BUILD≠HEAD.

---

## 1. Pendientes por hito

### Dependencias de fundación (10, 23)

- **10 — Migración de carga y reconciliación de registros:** completar loaders de compatibilidad, selección canónica del perfil geométrico y reconciliación de registros históricos; las suites de manifiestos ya pasan.
- **23 — Bancos pre-colisión:** ejecutar la matriz completa de referencia pre-colisión (equilibrio estático, freefall, heave, pitch, roll, bache, frenada, curva y suelo liso) con el camino de fuerzas de producción.

### M1 — Modelo físico del vehículo (items 07, 08, 11–20)

- **07 — Rangos de estimación y evidencia:** completar evidencia específica por componente y rangos de incertidumbre correlacionados; los valores actuales son estimaciones, no medidas.
- **08 — Centros y tensores completos:** verificar trayectorias, centros, sensibilidad y tensores agregados en todo el dominio operativo; validar cuerpos analíticos y esquinas espejadas.
- **11 — Hardpoints y envolventes:** barridos finitos de bump/rebound, dirección, heave, roll y combinaciones contra cárter, caja, transmisiones, miembros adyacentes y carcasa; el test actual del amortiguador trasero es solo de piel superior.
- **12 — Poses y derivadas espaciales:** envolventes de derivadas, convergencia, varios tamaños de diferencia finita, traslación/rotación separadas y continuidad de rama geométrica.
- **13 — Matriz de masa de producción:** comparación contra referencia, autovalores generalizados y condicionamiento en todo el estado; factorización reutilizable solo con configuración y masa sin cambios.
- **14 — Orientación e inercia rotatoria:** convergencia de energía de rotores girando y dirección prescrita en todo el dominio; normalización de cuaternión y signos giroscópicos.
- **15 — Fuerzas de suspensión por trabajo mecánico:** derivadas de fuerza y validación de energía con cargas combinadas; preload, amortiguador asimétrico y antiroll.
- **16 — Topes de recorrido como eventos:** unificar freno, recorrido y colisión; validar cruces simultáneos, restitución y trabajo de corrección.
- **17 — Cargas de articulación y montaje — COMPLETADO CANDIDATO:** revisar el informe delantera/trasera, la aproximación del radio de contacto y la tolerancia del modo promedio de paso.
- **18 — Giro de rueda, freno y transmisión:** completar momento/energía de motor, ejes y embrague; validar restricciones simultáneas.
- **19 — Neumático conectado al contacto real:** consultas de huella, rigidez vertical dependiente de presión, diagnóstico de deflexión y trabajo de deslizamiento.
- **20 — Aero y fuerzas externas:** reconstrucción real del suelo, trabajo de cada fuerza y diagnósticos de consumidores; respetar la referencia de fondo y las estaciones reales de eje del sprint aero.

**Gate M1:** gates de fundación verdes + derivadas con reporte + cierre de energía/momento dentro del presupuesto aprobado. No se declara superado.

### M2 — Contacto y mundo físico (items 24–35)

- **24 — Librería geométrica:** Parry 0.23.0 está fijado como candidato; falta la revisión formal de licencia, features, mantenimiento y cobertura medida.
- **25 — Geometría física del mundo:** paquete Fuji validado (74 mallas, 13.696 triángulos, hashes); falta promoverlo a la ruta de producción y validar cobertura/grupos de superficie completos.
- **26 — Formas de colisión del vehículo articulado:** cajas y cilindros actuales son estimaciones; falta fidelidad de formas, límites finitos y alineación en todo el recorrido/dirección.
- **27 — Consultas y filtrado de candidatos:** rayos, overlap y bounds anchos implementados; falta validar todos los dominios de consulta, bordes y paridad de geometría.
- **28 — Manifolds persistentes — COMPLETADO CANDIDATO:** revisar constantes de política, regla de costura y el hecho de que Parry devuelve manifolds con separaciones muy superiores a la predicción (filtradas por el mundo). Diez tests; el paso de reposo sobre plataforma converge con el presupuesto de 100 iteraciones.
- **29 — Impulsos normales — COMPLETADO CANDIDATO:** solver de conjunto activo (pivotes `2n+20`), regularización para soportes redundantes, restitución y fricción proyectada reconciliadas hasta 40 pasadas. Revisar el fallback de regularización y el presupuesto.
- **30 — Fricción tangencial — COMPLETADO CANDIDATO:** bloque 2×2 exacto, cono, stick/slip, mapa de impacto por superficie y warm start remapeado. **El perfil de producción mantiene el mapa vacío y usa 0.5 global** hasta calibración (item 47).
- **31 — CCD — COMPLETADO CANDIDATO:** cast no lineal de poses reales (incluye rotación y recorrido), diagnósticos de tiempo/forma/entidad/subdivisiones/fallos geométricos y tests de no-túnel a 50 m/s y con rotación. Falta la cota de error de trayectoria articulada y la matriz completa de impactos (item 43).
- **32 — Clasificación — COMPLETADO CANDIDATO:** partición tread (`normal.y >= 0.5`) vs impacto rígido (`wheel_impact_shape`) vs tope interno; eventos con propietario y contadores de duplicados. Faltan curbs compuestos, sidewall, bottoming y continuidad de fuerzas (item 43).
- **33 — Vehículo-vehículo — COMPLETADO CANDIDATO:** matriz normal corregida para emparejar entidades por identidad; islas y orden determinista; cadena de tres coches y paso a dos coches. **Pendiente:** con restitución/fricción de producción y un par de 4 contactos el solver alternante se queda en ~2e-4 m/s; el test de mundo aísla con restitución 0 y fricción 1e-6. Validar frontal/trasero/oblicuo/rueda-rueda/tres coches/montonera en item 43.
- **34 — Objetos dinámicos — COMPLETADO CANDIDATO:** conos de pista y barreras portátiles son propiedad del mundo Rust, comparten identidad y paso fijo, y sus contactos con vehículos, terreno y otros objetos usan el mismo solver generalizado con intercambio de momento. Ciclo de vida register/remove/reset/snapshot e interfaz nativa aditiva. Vehículos legacy de Godot y clases móviles desconocidas se rechazan explícitamente. Revisar el conjunto de clases soportadas, las formas proxy, masa/inercia y el emparejamiento de materiales; falta la creación/presentación en Godot.
- **35 — Reposo estable — COMPLETADO CANDIDATO:** manifolds persistentes con pasos de reposo y warm start completo; el solver reporta contactos estáticos y deslizantes; los contactos estáticos reciben una corrección tangencial de posición acotada que elimina la deriva de la velocidad previa al solve sin congelar la suspensión. Los objetos dinámicos duermen tras un intervalo de bajo movimiento con soporte y despiertan por contacto móvil o pérdida de soporte; los vehículos nunca duermen. CCD omite pares en contacto persistente para evitar tormentas de micro-impactos. Validación de costuras y energía de corrección acotada. Revisar umbrales, constantes y el recorte de CCD; faltan stacks/montoneras, fricción de larga duración, curbs y calibración de materiales.

**Gate M2:** matriz de impacto/contacto (item 43): morro contra asfalto sin volcar/saltar, golpe lateral acotado, piano parcial, aterrizaje, reposo sin jitter, sin túnel a 300 km/h y pasividad completa. No se declara superado.

### M3 — Integrador y coste de carrera (items 22, 46)

- **22 — Integrador de producción:** método/frecuencia con convergencia suave e impacto separadas, pasividad y tiempo físico nunca descartado en silencio.
- **46 — Coste de carrera:** cuellos de botella de geometría/matriz, mediana y cola en las 50 posiciones de Fuji, sin asignaciones en el paso. El muestreo de 32 coches sigue ~66× el intervalo; el nuevo solver de conjunto activo/2×2/40 pasadas **no tiene benchmark de parrilla**.

**Gate M3 (propuesta, pendiente de aprobación):** a 120 Hz, media ≤30% del intervalo para 20 coches y p95 ≤50%; 50 coches ≤60%; benchmark reproducible con BUILD/HEAD y hardware.

### M4 — Runtime, scheduling y activación (21, 36–38, 41, 42)

- **21 — Mundo Rust autoritativo:** implementación candidata; falta cerrar propiedad de integración y estado dinámico soportado.
- **36/37 — Interfaz nativa y paso único — COMPLETADOS CANDIDATOS:** revisar rebuild de la extensión de producción y la escena canónica completa; el harness nativo registró errores de telemetría/autoload.
- **38 — Ciclo de vida:** reset, pausa, recomposición tardía, pit stop y recuperación de fallos numéricos.
- **41 — Consumidores de gameplay/configuración:** tuning runtime, animación de boxes, cámaras, editores y recomposición de sesión.
- **42 — Activación y compatibilidad:** rebuild de producción, smoke canónico, rendimiento y aceptación humana; mantener el propietario por defecto sin cambios hasta aprobación.

**Gate M4:** `-PhysicalWorldPackage` en escena canónica con paridad de fuente, un solo avance por frame, pausa sin tiempo, servicios verificados y sin errores de autoload/telemetría.

### M5 — Presentación, telemetría y audio (39, 40)

- **39:** renderizar estado mecánico resuelto con interpolación y revisión visual humana.
- **40:** telemetría/eventos de colisión a audio, volante, HUD y todas las clases de evento; cooldown 0.2 s y raspado sostenido.

**Gate M5:** revisión visual del coche completo, audio sin artefactos, HUD coherente y cero solves de linkages.

### M6 — Paridad y sensibilidad (44, 45)

- **44:** replay/runtime byte-idéntico a iguales entradas y paso.
- **45:** incertidumbre correlacionada de centros/tensores y sensibilidad al paso temporal.

### M7 — Calibración, build limpio y publicación (43, 47–50)

- **43 — Matriz completa de contacto/impacto:** **pendiente crítico**, incluye los escenarios de los items 30–33, curbs, sidewall, bottoming, multi-coche y energía.
- **47 — Calibración completa:** altura, neumático, manejo, cargas de reacción, respuesta de contacto y mapa de materiales de impacto.
- **48 — Build limpio:** Cargo/SCons/Godot/DSP, huella de fuentes sucias, launcher canónico.
- **49 — Conducción integrada en Fuji:** smoke nativo, revisión visual trasera y aceptación humana.
- **50 — Auditoría y publicación atómica.**

**Gate final:** todos los gates con evidencia fechada y SHA; `human_approved` en el backlog; activación por defecto solo con aprobación explícita.

---

## 2. Hallazgos nuevos y bloqueantes

1. **Convergencia con fricción/restitución de producción en pares multi-contacto (item 33):** el solver alternante normal↔fricción se estanca alrededor de `2e-4 m/s` con un par de 4 contactos; el test de mundo usa restitución 0 y fricción `1e-6` para aislar la isla. No se puede declarar cerrada la matriz multi-coche hasta resolverlo o aprobar un presupuesto.
2. **Matriz de impacto/contacto (item 43) sin ejecutar:** no hay evidencia de morro contra asfalto sin volcar, golpe lateral, piano, aterrizaje, reposo sin jitter, 300 km/h, curbs compuestos, sidewall ni bottoming.
3. **CCD articulado (item 31):** el cast es un movimiento rígido no lineal entre poses extremas; falta la cota de error de trayectoria articulada y la validación de eventos simultáneos.
4. **Clasificación de curbs (item 32):** la partición por normal está implementada, pero la continuidad de transición en mallas compuestas y la fuerza de bottoming no están validadas.
5. **Materiales de impacto (item 30):** el mapa por superficie está implementado y probado, pero el perfil de producción está vacío; usa 0.5 global. La calibración por material es item 47.
6. **Rendimiento (item 46):** no hay benchmark de percentiles ni de las 50 posiciones de Fuji con el solver actual; el muestreo previo de 32 coches era ~66× el intervalo. El coste del LCP de conjunto activo + bloque 2×2 + 40 pasadas no está medido.
7. **BUILD/HEAD:** `game/BUILD_SOURCE = ce0f153c` frente a HEAD `29d9fc52`; cualquier ejecución canónica será rechazada hasta rebuild. El cambio aero/fondo ya está commiteado en HEAD, pero no sellado.
8. **Trabajo sin commitear:** toda la física de los items 17 y 28–33 y los documentos de estado están en el árbol de trabajo; requieren revisión y aprobación antes de commit. Los cambios ajenos de audio y `recorded_engine_profile.rs` deben preservarse.
9. **Fallos preexistentes de la suite (no perseguir):** 14 fallos en lib aero (1), `aero_test` (4), `fuel_model_test` (1), `regression_suite_test` (1) y `suspension_multibody_test` (7, cierre de inventario sintético). No son regresiones de esta continuación.
10. **Diagnóstico `refresh_contact_manifolds`:** avanza el contador de paso y la edad de la caché sin tiempo físico; es una API de diagnóstico, no una ruta de scheduling.
11. **Topes de recorrido:** siguen siendo restricciones internas del modelo de suspensión; el “wheel-at-limit” es un diagnóstico, no un evento de mundo.
12. **Objetos dinámicos (item 34):** solo conos y barreras Rust están soportados. Las formas son proxies (cilindro/caja) y la masa/inercia son estimaciones; la fricción objeto-terreno usa el código del terreno y la de pares usa el código del objeto. No hay creación, render, audio ni replay de objetos en Godot. Los vehículos legacy siguen rechazados. El reposo estable, sleep/wake y stacks pertenecen al item 35.
13. **Reposo y sleep (item 35):** los umbrales de reposo, el retardo de sueño, la velocidad de despertar y el salto de CCD en contactos persistentes son constantes candidatas, no calibradas. El sueño mantiene un objeto quieto aunque la fricción estática no se haya validado en esa pose; la fricción estática se validó despertando el objeto en la pendiente. Faltan stacks/montoneras, reposo sobre curbs y paredes, y calibración por material. No hay visualización ni audio del estado dormido en Godot.

---

## 3. Gates de fundación (sin cambios)

De `docs/coupled_vehicle_validation_specification.json`:

- `existing_reference_tests`: 28 pases esperados en las 3 suites declaradas.
- 9 gates de fundación y 9 de desarrollo futuro con `must_not_claim_pass_without_report`.
- Fixture: `game/data/vehicles/f1_2030/f1_2030_v10_geometric.json`, definición/escenario de fundación y clasificación `five_component_estimate_diagnostic_only`.
- La falla geométrica conocida se mantiene atribuida; no ocultarla.

## 4. Checklist de rebuild, ejecución y entrega

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
- Cualquier commit mueve HEAD: **rebuild obligatorio** antes de lanzar.
- Registrar BUILD/HEAD, hardware y perfil en cada evidencia.
- Preservar los cambios ajenos de audio y `recorded_engine_profile.rs`.

## 5. Decisiones abiertas para el humano

1. **Commit/revisión de la física candidata:** aprobar los items 17 y 28–33 y sus cambios sin commitear; después rebuild para sellar BUILD/HEAD.
2. **Convergencia multi-contacto con fricción/restitución de producción:** ¿se corrige el solver acoplado antes de la matriz de impacto o se aprueba un presupuesto/tolerancia documentado?
3. **Backend `lite`:** ¿se cancela o se mantiene como segundo nivel?
4. **Presupuesto de rendimiento M3:** aprobar la propuesta de §1-M3 y medir el nuevo solver.
5. **Altura estática del Williams** (30/45 mm documentados vs +50 mm actual) y residual de drag (~6–12% bajo el CdA).
6. **Divergencia de barras** entre backends (simplified 1.00/0.80 vs standard 0.15/0.09).
7. **Mapa de materiales de impacto:** valores por superficie y calibración (item 47).
8. **Sprint aero/fondo:** el commit ya existe en HEAD (`29d9fc52`); falta rebuild/reseal de `BUILD_SOURCE`.

## 6. Límites

- El modo `standard` del motor de vehículo debe conservar ecuaciones, referencias, contactos y masa.
- No seleccionar correcciones por nombre de coche ni aplicar cambios globales al perfil compartido para corregir un solo coche.
- Ninguna nota de este documento autoriza saltarse gates fallidos ni marcar los 50 items como completos.
- La activación por defecto del mundo acoplado exige aprobación humana explícita; hasta entonces permanece opt-in.
