# Formula 2030 — Backlog para físicas alternativas de bajo consumo (`main-clean-v2`)

**Base auditada:** [`Anthony38zrgo-2/formula-90`, rama `main-clean`](https://github.com/Anthony38zrgo-2/formula-90/tree/main-clean), revisión del 15/10/2026.  
**Estado de este documento:** propuesta de implementación, **no código ejecutado ni rama creada**.  
**Resultado perseguido:** un segundo backend de física `lite`/`v2`, más rápido y con una sensación de conducción verosímil próxima a la escuela rFactor 1 / GTR2 / Automobilista 1. Mantener el comportamiento físico causal y no perseguir una representación estructural tipo BeamNG.

## 0. Decisión arquitectónica y límites

No reescribir ni borrar el backend actual. Mantener dos ejecuciones:

- **`standard`**: implementación actual, referencia A/B y regresiones.
- **`lite`**: nuevas ecuaciones y aproximaciones para neumáticos, suspensión y aerodinámica; térmicas multirritmo. Comparte contratos de entrada/salida, telemetría, audio, volante y lógica de carrera.
- **`coupled_candidate`**: prototipo Rust de dinámica multicuerpo y mundo físico acoplado; queda apartado del runtime de `lite`. No cargar sus masas por componente, matrices generalizadas, cálculo de trayectorias completo ni solver de colisiones como dependencia de este backend.

El control debe permitir decidir `standard` o `lite` **al crear el vehículo**, sin duplicar entradas de HUD, sonido, telemetría o código de carreras. Un coche recibe exactamente una autoridad de integración. Mantener la interfaz estable de fuerzas y torques (`ForceTorqueOutput`) y el esquema de estado/telemetría existentes; los campos que no sean físicamente significativos en `lite` deben definirse como derivados o indisponibles, nunca rellenarse con números engañosos.

**Importante:** crear una implementación alternativa no garantiza ahorro si internamente sigue ejecutando todos los módulos `standard` y solo reemplaza su salida. `lite` debe **omitir el trabajo costoso**, no ejecutarlo y luego ignorarlo.

**No objetivos:** CFD, deformación estructural, cálculo por tick de 80 cuerpos mecánicos, FE de neumáticos, resolver cada articulación de brazos y tirantes, simulación de circulación de fluidos, migración de toda la arquitectura de colisiones a Rust, pérdida deliberada de transferencia de carga, controles artificiales obligatorios o eliminar las propiedades dinámicas del diferencial.

### Contrato conceptual

```text
Input / timing / collisions
          |
  VehiclePhysicsMode
    /            \
standard        lite
  |               |
Current Rust   Lite Rust
solver         tire/susp/aero/thermal
  \             /
  ForceTorqueOutput + TelemetryFrame
                |
Godot rigid body + cameras + HUD + audio + AI
```

> El diagrama representa el contrato lógico. No se deben introducir dos integradores que actualicen simultáneamente el mismo vehículo. El modo de mundo físico acoplado continúa como ruta experimental separada.

### Dónde integrar

**Archivos existentes examinados:**

- [`game/crates/vehicle-physics-engine/src/simulation.rs`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/vehicle-physics-engine/src/simulation.rs) — `VehicleSimulator`, `solve_forces`, avance térmico, ayudas, FFI externa.
- [`.../src/tire.rs`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/vehicle-physics-engine/src/tire.rs) — combined slip, transitorios, fuerzas.
- [`.../src/suspension.rs`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/vehicle-physics-engine/src/suspension.rs) — contacto tricast y mecánica virtual.
- [`.../src/suspension_kinematics.rs`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/vehicle-physics-engine/src/suspension_kinematics.rs) — solucionador geométrico.
- [`.../src/aero.rs`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/vehicle-physics-engine/src/aero.rs) — fuerzas aerodinámicas y límites.
- [`.../src/tire_thermals.rs`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/vehicle-physics-engine/src/tire_thermals.rs), [`brake_thermals.rs`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/vehicle-physics-engine/src/brake_thermals.rs), [`powertrain_thermals.rs`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/vehicle-physics-engine/src/powertrain_thermals.rs), [`tire_wear.rs`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/vehicle-physics-engine/src/tire_wear.rs).
- [`.../src/vehicle_configuration/loader.rs`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/vehicle-physics-engine/src/vehicle_configuration/loader.rs), [`.../src/ffi.rs`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/crates/vehicle-physics-engine/src/ffi.rs), `native/src/core/f90_core.cpp`, `native/src/vehicle/f1_94_rust_vehicle.cpp` — carga de perfil y API Godot.
- [`game/scripts/vehicle/rust_vehicle_controller.gd`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/scripts/vehicle/rust_vehicle_controller.gd) — 12 raycasts y telemetría.
- [`game/data/vehicles/f1_2030/profiles/geometric/vehicle.json`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/game/data/vehicles/f1_2030/profiles/geometric/vehicle.json) — manifiesto modular.

**Rutas NUEVAS sugeridas** (no afirmar que existen):

```text
game/crates/vehicle-physics-engine/src/lite/
  mod.rs                  # selección e interfaz estable del solver
  tire.rs                 # modelo causal reducido
  suspension.rs           # fuerzas + LUT de cinemática
  suspension_lut.rs       # construcción y evaluación validadas
  aero.rs                 # 3 contribuciones + drag
  thermal.rs              # acumuladores de energía + tick lento
  diagnostics.rs          # invariantes y trazas A/B
  config.rs               # datos Lite con defaults explícitos

game/crates/vehicle-physics-engine/tests/
  lite_tire_test.rs
  lite_suspension_lut_test.rs
  lite_aero_test.rs
  lite_thermal_test.rs
  lite_standard_parity_test.rs
  lite_timestep_convergence_test.rs

game/data/vehicles/f1_2030/profiles/lite/
  vehicle.json            # perfil derivado/sincronizado de geometric
  ...                     # únicamente los dominios que realmente cambien

tools/physics_diagnostics/
  compare_standard_lite.py
  benchmark_lite_grid.py
  calibrate_lite_lut.py

docs/
  physics_lite_design.md
  physics_lite_calibration.md
  physics_lite_benchmark.md
```

No agregar `physics_mode` a JSONs arbitrariamente sin actualizar el esquema y su parser: el proyecto valida dominios y campos. Preferir un selector de ejecución externo al perfil físico inmutable o un campo opcional versionado en el contrato de configuración. Las rutas relativas entre manifiestos deben respetar `authorized_root`; no presuponer que `../geometric/` está permitido sin leer la validación real.

## P0 — Referencia obligatoria y criterios de aceptación

**P0.1. Inmovilizar referencia.** Registrar SHA de `main-clean`, manifiesto exacto, datos de masa, combustible, marchas, neumáticos y asistencias. Usar el perfil modular geométrico actualmente escogido por la escena. Desactivar la ayuda de guiñada en pruebas de referencia (el perfil geométrico actual tiene `stability_default_enabled: true`) y dejar TC/ABS desactivados, o ejecutar matrices separadas con ayudas explícitas. Registrar que el perfil geométrico inspeccionado declara aproximadamente 678,34 kg de masa seca y 7,6 kg iniciales: no mezclar con documentos anteriores que hablan de 698 kg.

**P0.2. Banco determinista.** Grabar secuencias de mando, poses, raycasts/contacto y contexto de superficie para repetir sin piloto humano: aceleración, frenada con y sin giro, radio constante, step-steer, lift-off, transición asfalto/piano, trazado Fuji y entrada/salida de boxes. Semilla fija, frecuencia fija, misma masa/temperatura/combustible y misma fuente de integración y colisiones.

**P0.3. Instrumentar CPU por fase.** `time/tick` y distribución p50/p95/p99 para neumáticos, suspensión, cinemática, aero, estados lentos, FFI/native, raycasts/terreno, colisiones y presentación. Construcción release, hardware fijo, pausas de GC/compilación fuera de la medición, 3–5 repeticiones y calentamiento previo. Medir 1, 4, 12 y 20+ coches; 120 y 240 Hz. **La suma de tiempo por función de un solo coche no sustituye un perfil de parrilla completa.**

**P0.4. Establecer objetivo cuantitativo.** Propuesta de gate inicial: **≥35% de reducción de CPU en el paso físico por coche a la misma frecuencia y escenario** respecto de `standard`; objetivo de aspiración 50%. Exigir además mejora real del p95 de parrilla. La cifra es un umbral de proyecto, no una ganancia medida. Para 120 Hz el intervalo completo es 8,33 ms; para 240 Hz 4,17 ms. Deben caber también IA, juego y presentación: no asignar el intervalo entero a la física.

**Aceptación P0:** línea de base CSV/JSON reproducible; usuario puede distinguir los modos; modo `standard` no se altera; diagnósticos y referencias quedan guardados.

## P1 — Neumáticos reducidos

**Objetivo:** preservar pérdida y recuperación de agarre, transferencia de carga, aceleración/frenada simultánea con giro y un deslizamiento controlable, pero retirar duplicación temporal y parámetros superpuestos.

### Estado del código

`tire.rs` calcula slip geométrico, slip efectivo mediante relajación, escala de grip sensible a carga/estado, curvas post-pico, migración de máximos, dos memorias post-pico, presupuesto combined-slip, mezcla de contacto y momento autoalineante. En el perfil geométrico actual, el `min_relaxation_tau_s` es 0,045 s; `slip_loss_tau_s` 0,06 s; `slip_recovery_tau_s` 0,25 s; `contact_ramp_tau_s` 0,06 s; y las migraciones de pico son 0,35 y 0,25. Son múltiples fuentes de fase/transitorio y necesitan justificación individual.

### Diseño Lite

Por rueda conservar: `spin`, `slip_angle`, `slip_ratio`, **un estado relajado por cada eje** (si los ensayos muestran que se requiere), `Fz`, `Fx`, `Fy`, `Mz`, temperatura efectiva, presión/grip efectivo y estado de contacto. Mantener:

1. Curva longitudinal y lateral con **pendiente antes de pico, pico y plateau/falloff** coherentes.
2. Sensibilidad de adherencia a carga vertical; no asumir `F = mu_0 * Fz` lineal para todas las cargas.
3. Presupuesto Fx/Fy compartido, con continuidad en la transición a saturación.
4. Relajación física controlada por **longitud** o un tiempo equivalente derivado de velocidad, sin duplicar retardos ajustables que actúen sobre el mismo evento.
5. Camber (preferiblemente como un pequeño desplazamiento calibrable de respuesta lateral) y cambio de superficie; conservación de radio efectivo suficiente para dinámica/odometría.
6. Par autoalineante aproximado por trail que caiga hacia la saturación; si no existe FFB, conservar por coherencia/telemetría, con opción de actualizar solo si hay consumidor real.

Eliminar **solo en Lite** por experimentos A/B:

- Fase A: `post_peak_decay_lat/lon` como filtros independientes; usar caída directamente de la curva estacionaria. No introducir parámetros nuevos para imitar su efecto.
- Fase B: `combined_lateral_peak_migration` / `combined_longitudinal_peak_migration`; usar la misma combinación para todo el rango.
- Fase C: asimetría de recuperación adicional (`relaxation_recovery_scale`) si el estado relajado simple conserva estabilidad y sensación de recuperación.
- Fase D: duplicación entre `contact_ramp_tau_s`, filtrado de altura/carga de suspensión y retardo de slip. **Una transición de contacto suficiente** para evitar impulsos; no filtrar la pérdida física de Fz dos veces.

**Procedimiento experimental obligatorio:** N0 estándar; N1 sin memorias post-pico; N2 sin migración de pico; N3 con solo relajación simétrica; N4 con contacto reducido. Cada paso se evalúa por separado y el resultado aprobado es aquel que no causa quiebres, snaps ni cambios inexplicables. Registrar las curvas Fx(kappa,Fz), Fy(alpha,Fz), combined Fx/Fy y restitución tras slide, para Fz bajo/normal/alto, seco y piano.

**No eliminar:** combined slip, sensibilidad a Fz, caídas después del pico, fuerzas negativas correctas en frenado y subpaso numérico donde sea necesario. La simplificación no debe transformar el neumático en un agarre constante por rueda.

**Pruebas sugeridas:**
- Sobre asfalto y condiciones estabilizadas, objetivos A/B iniciales de fuerza máxima y aceleración lateral dentro de **±5%**; no validar solo contra el `standard` si este contiene defectos.
- Sin saltos de fuerza no físicos alrededor del pico; continuidad de Fx/Fy cuando se mezclan aceleración y giro.
- Cambio de paso 120→240 Hz: discrepancia menor que el presupuesto medido y aprobado, inicialmente <3% en fuerzas integradas y yaw bajo maniobras regulares.
- Acelerador al 100% a baja velocidad: transiciones suaves y wheelspin físicamente visible, sin TC oculto.
- Al salir de un slide moderado, agarre recuperable sin retraso persistente que no tenga origen físico.

**Entregable P1:** `lite/tire.rs`, curvas de comparación, suites de bajo nivel y lista explícita de parámetros retirados y conservados. El código `standard` permanece inalterado.

## P2 — Suspensión y contacto ligeros

**Objetivo:** preservar la transferencia de carga y el trabajo del amortiguador sin evaluar continuamente toda la geometría de doble triángulo / pushrod / pullrod ni matrices multicuerpo.

### Diseño

**P2.1. Modelo de cuatro esquinas desacopladas estructuralmente**, acopladas por el único cuerpo rígido, barras estabilizadoras y transferencia de carga. Calcular resorte, amortiguador (bump/rebound y rodilla opcional), anti-roll, topes y Fz. Mantener un grado de libertad efectivo de rueda o contacto cuando haga falta por pianos: reducir la complejidad del solver, no ignorar el recorrido.

**P2.2. LUT geométrica offline:** usando la geometría real de `suspension_kinematics.rs`, construir por esquina una tabla muestreada en recorrido de rueda `q` y, si afecta a delantero, giro de rack `r`. Tabular posición de buje, camber, toe, motion ratio, amortiguador y desplazamiento del punto de contacto. Interpolación lineal o cúbica con seguridad de monotonía, bordes explícitos y valores no finitos rechazados. Probar malla inicial **65 muestras en recorrido** y entre **9–17 para dirección delantera**; aumentar solo si la desviación lo requiere. *Estas cifras son propuestas de diseño, no parámetros medidos.*

**P2.3. Error LUT** frente a solucionador de referencia en toda la envolvente: posición del buje <1 mm, camber/toe <0,1°, motion ratio <2% relativo o tolerancia absoluta documentada cerca de cero. La tasa/sentido de cambio deben conservarse. Cachear el resultado en el archivo de perfil, ligado a digest de hardpoints/versión/escala/recorrido. Si cambia la geometría, invalidar/reconstruir offline: nunca usar una tabla obsoleta.

**P2.4. Contacto por fidelidad:** base Lite **un raycast central por rueda** sobre asfalto y pruebas regulares. Modo opcional de tres rayos donde sea importante recuperar el plano local del piano y contacto parcial, preferentemente activable para el jugador durante validación. No prometer equivalencia de 1 y 3 rayos al subir un piano asimétrico: medir el comportamiento, ofrecer modo de contacto detallado si el ahorro no compensa. Las señales de audio y el contacto con muro/fondo plano se gestionan por separado.

**P2.5. Física visual:** reutilizar LUT para articulación visual de suspension/rueda, evitando un solucionador completo paralelo en GDScript/C++; esto no debe cambiar la geometría física y visual de forma contradictoria.

**P2.6. Protección numérica:** equilibrio a reposo, topes, vibración del amortiguador, carga positiva unilateral, restitución de contacto, simetría izquierda/derecha, cambios de superficie, transitorios de piano y aterrizaje. Limitar subpasos al número que exija estabilidad, no por una frecuencia arbitraria exagerada.

**Aceptación P2:** carga estática por rueda ±2% frente al equilibrio de referencia; altura ±3 mm; respuesta de step-bump sin crecimiento de energía espurio; cero NaN; contacto unilateral; desviaciones de maniobra corregidas con física, sin yaw assist; mejora medible de CPU por suspensión frente al estándar.

**Entregable P2:** `lite/suspension.rs`, `lite/suspension_lut.rs`, generador y prueba de invalidez de cache; vídeos/trazas A/B de piano y transferencia de carga.

## P3 — Aerodinámica reducida

**Objetivo:** downforce, drag y reparto físicamente causales con bajo coste constante por vehículo.

### Modelo Lite de tres elementos

Se calcularán tres contribuciones principales y el drag total, **una vez por paso del vehículo**:

- Ala delantera: carga `Df(v,incidencia)` aplicada en zona delantera.
- Ala trasera: carga `Dr(v,incidencia)` aplicada atrás.
- Fondo plano: `Dfloor(v,h,rake)` con penalización suave por altura demasiado baja/alta y por pitch/rake fuera de rango.
- Drag = carrocería + ala/fricción inducida simplificada + fondo/ruedas cuando sea relevante; evitar doble conteo de superficie de referencia.

Esquema: `q=0.5*rho*v_air^2`, `D_i=q*S_i*Cl_i*f_height/f_yaw`, `Drag=q*CdA_effective`. No introducir un `Cl` aerodinámico libre adicional que el usuario tenga que ajustar además de la carga calibrada; agrupar en datos fijos derivados de curvas existentes. Mantener fuerza y momento de aplicación en posiciones del chasis, de modo que cambie el reparto de carga real. El viento debe usar velocidad relativa al aire, no la del vehículo en suelo.

**Aplazar:** estados de respuesta separados para cada ala, un gran número de polares editables, mapa denso de suelo, separación de flujo por mitad lateral, coeficientes de sellado y exponentes sin efecto demostrable. Permitir una pérdida suave de eficiencia del fondo pero sin retardos extra salvo que mejoren una comparación documentada.

**Topes aerodinámicos:** cuando se active `soft_limit` o `hard_limit`, emitir `raw_downforce`, `limited_downforce`, `limit_factor` en telemetría. No ajustar el coche para que el límite duro esté saturado habitualmente; hacerlo indicaría calibración incorrecta u ocultación de un error de Cl/áreas.

**Matriz de verificación:** velocidades 80/160/240/300 km/h, alturas baja/normal/alta, rake negativo/neutro/positivo y yaw moderado. En zona de funcionamiento normal, objetivo A/B inicial: downforce total ±5%, balance aero ±3 puntos porcentuales, drag y velocidad máxima dentro de presupuesto acordado; fuera de la zona normal puede divergir si desaparece una no linealidad injustificada, con aprobación por pruebas subjetivas.

**Aceptación P3:** el comportamiento de subviraje/sobreviraje responde a cambios delantero/trasero; `D~v²` donde corresponde; altura afecta gradualmente al fondo; no hay cargas discontinuas o fuerzas al revés; menos tiempo de aero por tick.

**Entregable P3:** `lite/aero.rs`, presets derivados (no parámetros arbitrarios), trazas de balance de cargas, registro explícito de límites artificiales.

## P4 — Estados lentos: térmicas, desgaste y refrigeración

**Objetivo:** conservar estrategia, degradación, calentamiento y efectos sobre prestaciones evitando resolver una red térmica amplia en cada subpaso mecánico.

### Regla: separar producción de energía y actualización de temperaturas

En **cada tick físico**, continuar acumulando energías y estados de operación: `E_brake += brake_torque*angular_speed*dt` con signo físico/disipación corregidos, trabajo de deslizamiento `E_tire += max(0,-F·v_slip)*dt`, potencia/carga de motor, tiempo, velocidad y entorno. Los modificadores mecánicos (grip, torque derating, brake fade) se consultan desde el estado térmico vigente, **sin ejecutar cada tick toda la red térmica**.

Cada `0,05 s` para neumáticos (20 Hz) y `0,1 s` para frenos, agua y aceite (10 Hz) como propuesta inicial, integrar las energías acumuladas usando el `dt` real total del intervalo, actualizar temperaturas y límites. No promediar incorrectamente solo el último tick. Si el backend detecta un evento extremo (frenada severa, bloqueos sostenidos, daño o transición), permitir una evaluación anticipada. Verificar temporalidad y ajustar frecuencia según error.

### Simplificaciones

**P4.1. Neumáticos:** un nodo térmico de banda efectiva + carcasa y gas/ presión cuando aporte consecuencias físicas, con **temperatura/presión/desgaste propios de cada rueda**. Temperaturas inner/middle/outer pueden seguir existiendo para presentación como estimaciones derivadas de camber/condiciones; no afirmar que son tres nodos calculados si no lo son. Evitar tres redes térmicas por banda si no cambian la estrategia de manera identificable.

**P4.2. Desgaste:** un escalar por rueda o profundidad nominal que responde a trabajo de deslizamiento, temperatura y superficie. Mantener diferencias delanteras/traseras. Evitar fluctuaciones rápidas de desgaste por frame. Permitir la caída progresiva y una penalización al final de vida solo si existe un calibrado tangible.

**P4.3. Frenos:** uno o dos nodos equivalentes por esquina (disco efectivo y masa secundaria solo si se verifica transferencia a la llanta/neumático). Fade según temperatura, enfriamiento con velocidad y duct apertura, transferencia térmica al neumático agregada. Conservar potencia de freno y brake bias de forma instantánea.

**P4.4. Motor:** mantener par vs RPM, inercia, freno motor, combustible, cambio y delta de temperatura; aceite/agua pueden usar dos estados térmicos de primer orden, refrigeración agregada por conductos y torques reducidos por sobrecalentamiento. La simulación del bloque como nodo adicional queda opcional; si se elimina, comprobar transitorios térmicos de encendido, plena carga y vuelta de enfriamiento.

**P4.5. Estados externos:** presión y temperatura deben continuar alimentando el neumático, temperatura de freno el torque efectivo y temperatura del motor el torque disponible. Interpolar o suavizar entre actualizaciones térmicas cuando produzcan saltos perceptibles; sin añadir múltiples tiempos de respuesta editables redundantes.

### Verificación

- Energía generada, transferida, disipada y acumulada con balances y límites físicos; no calentar ruedas sin trabajo/causa.
- Sesiones de 1, 5 y 20 vueltas, sin tuning en marcha, comparando evolución de grip, presión, desgaste y temperaturas con referencia `standard` (y después datos externos cuando existan).
- Objetivos preliminares: temperatura efectiva en régimen ±5 °C frente a referencia calibrada, grip medio y consumo por vuelta dentro de ±3–5%, sin cambios bruscos en fade ni degradación. Estos son gates iniciales ajustables tras fijar el modelo verosímil.
- Compatibilidad de pitstop, reinicio, reemplazo de neumáticos, cambio de combustible, estado de HUD, FFB, telemetría y replays.
- Ensayar periodos `0,05/0,10/0,20 s` frente a actualización plena y seleccionar frecuencia con menor coste aceptable.

**Entregable P4:** `lite/thermal.rs`, estados equivalentes, acumuladores de energía, pruebas de integridad térmica y serie de carrera prolongada.

## V1 — Calidad, rendimiento y sensación de conducción (antes de publicar la rama)

| Familia | Prueba | Gate preliminar |
|---|---|---|
| Control | 20 repeticiones de aceleración/frenada/step steer con mismos inputs | reproducible, sin NaN ni cambios no explicados |
| Grip | círculo de aceleración, radio fijo, freno+giro y recuperación post-pico | curvas continuas y no contradictorias; fuerza pico ±5% de objetivo calibrado |
| Suspensión | asfalto, bache, piano parcial, salto, final de recorrido | carga unilateral y estabilidad, sin energía generada espuria |
| Aero | 4 velocidades × 3 alturas × rake y yaw | tendencia y reparto verificables, sin topes saturando normalmente |
| Estados lentos | 1/5/20 vueltas más reset/pitstop | balances de energía, estado persistente coherente |
| Multifrecuencia | 120, 240 y 480 Hz frente a convergencia alta | sensibilidad controlada, no comportamiento distinto por FPS |
| Integración | Godot, FFI, audio, HUD y telemetría | sin cambios de interfaz ni estado inconsistente |
| Coste | 1/4/12/20+ vehículos y carretera representativa | ≥35% menos CPU de física/coche a igual frecuencia; p95 de parrilla mejor |
| Experiencia | mando + volante si se dispone, curva lenta, rápida, pianos y salida | corregible, progresivo, no reacciones artificiales obligatorias |

Los umbrales son **propuestas de aceptación**, no resultados ya medidos ni cifras oficiales de rFactor/GTR2/AMS1. Los resultados deben compararse con objetivos externos cuando existan y con conducción humana controlada, no únicamente con el estándar heredado. Si el estándar exhibe un defecto, no se debe copiar dicho defecto para conseguir paridad.

El benchmark debe separar **coste físico por coche** y **coste total de carrera**. El antiguo candidato acoplado registra en la documentación 1,3179 s de computación para 0,02 s físicos con 32 coches en una prueba corta: esa cifra no es directamente comparable con una build optimizada `lite`, pero demuestra por qué no puede seguirse usando el candidato completo como supuesto modelo económico.

## Secuencia de implementación y decisiones de parada

1. **P0 baseline** y activar selección de backend sin comportamiento Lite aún; los dos modos ejecutan `standard`. Prueba de igualdad exacta y no regresión.
2. Implementar **P1 neumático Lite** y calibrar. Si causa sensación deficiente, detener, aislar variable y revertir solo P1; no compensar por asistencia de yaw.
3. Implementar **P2 suspensión Lite**, LUT offline y modos de raycast. Perfilar geometría/FFI antes y después. No disminuir contactos si el coste dominante es otra cosa.
4. Implementar **P3 aero Lite**, comprobar downforce absoluto y reparto con telemetría y tests de pista.
5. Implementar **P4 térmicas Lite**, mantener las señales de carrera, someter a pruebas largas y de repetibilidad.
6. Ejecutar regresiones estándar, A/B y benchmark de 1 a 20+ vehículos; resolver fallos; guardar los artefactos de validación.
7. **Solo tras superar los gates**, publicar como rama `main-clean-v2`, sin reescribir ni mover `main-clean`.

## Estrategia Git segura y creación diferida de `main-clean-v2`

No desarrollar directamente sobre `main-clean` ni crear prematuramente la rama final. Realizar trabajo en **worktree** o rama técnica aislada creada desde un SHA inmutable de `main-clean` (preservando los cambios locales del usuario, si los hay). Ejemplo únicamente ilustrativo:

```bash
# Comprobar el estado primero y registrar el SHA exacto
git status --short
git rev-parse main-clean

# Usar una rama temporal; no se toca main-clean
git worktree add -b work/physics-lite ../formula90-physics-lite main-clean
cd ../formula90-physics-lite

# Implementar P0..P4 + pruebas + commits incrementales aquí.
# NO seguir si existe alguna modificación del usuario no protegida.

# Al superar todas las pruebas, crear la rama final en el commit validado
git branch main-clean-v2 HEAD
git push -u origin main-clean-v2
```

Antes del último comando, confirmar que `main-clean-v2` todavía no existe en remoto; en la consulta de ramas realizada durante esta planificación no se encontró esa rama. Si otra persona la crea entretanto, detenerse y comparar refs en vez de forzar la actualización.

**Criterios para promover:** cambios de P1–P4 completos, perfil Lite identificado, tests verdes/defectos documentados, benchmark reproducible, conducción revisada, `standard` intacto, firmware/ABI nativo alineado con el SHA fuente, sin bibliotecas antiguas ni binarios locales copiados como evidencia falsa. Marcar un commit/version de aceptación y desde él generar `main-clean-v2`.

## Backlog en forma de tickets

| ID | Trabajo | Dependencia | Evidencia necesaria |
|---|---|---|---|
| P0-01 | Capturar SHA, perfiles, masa y ayudas | ninguna | informe baseline |
| P0-02 | Replays deterministas + trazas | P0-01 | archivos de entrada y resultados |
| P0-03 | Timings por subsistema + parrilla | P0-02 | CSV p50/p95/p99 |
| P0-04 | Selector `standard`/`lite` sin segunda integración | P0-01 | equivalencia modo sombra |
| P1-01 | Curvas de neumáticos de referencia | P0-02 | plots Fx/Fy/Mz |
| P1-02 | Modelo combined-slip Lite | P1-01 | test de saturación, cargas y simetría |
| P1-03 | Ablación post-pico, migración y retardos | P1-02 | matriz N0–N4 |
| P1-04 | Calibración de steering/lift-off/frenada | P1-03 | trazas y aprobación subjetiva |
| P2-01 | LUT de geometría offline, digest e invalidación | P0-01 | comparación en dominio completo |
| P2-02 | Resortes/dampers/antiroll/contacto Lite | P2-01 | equilibrio y step-bump |
| P2-03 | Raycast central y comparación con tricast | P2-02 | piano y CPU A/B |
| P2-04 | Integración de visual/suspensión sin segundo solve | P2-01 | pose física y visual concordante |
| P3-01 | Modelo 3 elementos + drag, bajo perfil | P0-02 | matriz velocidad/altura/rake |
| P3-02 | Calibrar balance y pérdida de fondo | P3-01 | test rápida/lenta/frenada |
| P3-03 | Telemetría de límites aerodinámicos | P3-01 | no saturación crónica |
| P4-01 | Acumuladores de energía multirritmo | P0-04 | balances energéticos |
| P4-02 | Térmica de neumático simplificada | P4-01 | stint y presión por rueda |
| P4-03 | Térmica/fade frenos agregada | P4-01 | braking heat y fade |
| P4-04 | Motor y refrigeración de pocos nodos | P4-01 | torque/derate/consumo |
| P4-05 | Desgaste y pitstop/reset/replays | P4-02 | estado de carrera integrado |
| V1-01 | Tests de paridad y convergencia | P1–P4 | informe objetivo |
| V1-02 | Tests reales de manejo y FFB | V1-01 | acta + trazas |
| V1-03 | Bench 1/4/12/20+ y budget p95 | V1-01 | reporte repeatable |
| GIT-01 | Revisión de cambios/código SHA/binaries | V1-01..03 | release checklist |
| GIT-02 | Crear `main-clean-v2` desde SHA validado | GIT-01 | URL de rama final |

## Referencias y restricciones importantes

- Project Chrono: [`ChTMsimpleTire.cpp`](https://github.com/projectchrono/chrono/blob/main/src/chrono_vehicle/wheeled_vehicle/tire/ChTMsimpleTire.cpp) — modelo de fuerzas combinado semiempírico para handling. Referencia comparativa, no obligación de copiarlo.
- VDrift: [`cartire1.cpp`](https://github.com/VDrift/vdrift/blob/master/src/physics/cartire1.cpp), [`cartire2.cpp`](https://github.com/VDrift/vdrift/blob/master/src/physics/cartire2.cpp), [`cardynamics.cpp`](https://github.com/VDrift/vdrift/blob/master/src/physics/cardynamics.cpp) — base técnica para separar fuerzas de neumáticos de dinámica de chasis, **revisar GPL antes de copiar código**; inspirarse en arquitectura o matemáticas generales no requiere portar código.
- Formula 2030: [`docs/coupled_vehicle_candidate_implementation.md`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/docs/coupled_vehicle_candidate_implementation.md), [`docs/coupled_vehicle_godot_integration.md`](https://github.com/Anthony38zrgo-2/formula-90/blob/main-clean/docs/coupled_vehicle_godot_integration.md) — candidato experimental no certificado para parrilla completa.

**Política de calibración:** no introducir una variable nueva para corregir un problema producido por otra variable si puede corregirse la causa primaria. Cada variable pública de tuning debe tener unidades, intervalo validado, relación medible con el comportamiento y al menos una prueba de regresión asociada.
