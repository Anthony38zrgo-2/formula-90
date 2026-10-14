# Handoff: cerrar omisiones de parámetros en la migración de perfiles F1 2030

Fecha: 2026-10-12, America/Bogota. Workspace: `D:\Formula90s`. Shell: PowerShell.
Rama observada: `main-clean`.
HEAD observado: `dd3b181d03bfbeb08025baf8c07311fde492f11f`.

Este documento reemplaza completamente el handoff anterior. La segunda auditoría profunda debe realizarla quien recibe este handoff: todavía no se ha ejecutado. El encargo es auditar todos los cambios de la migración, confirmar y ampliar los hallazgos descritos, y preparar su corrección coordinada. No asumir que la revisión inicial cubrió todos los parámetros.

## 1. Procedencia y estado que debes verificar al empezar

Registrar rama, HEAD, remotos, índice y estado completo antes de editar. Releer los AGENTS.md aplicables. El checkout está muy sucio: conserva cambios visuales previos, el `.blend`, GLB, manifiestos, eliminaciones del vehículo 2026, importaciones y binarios generados. No cambiar de rama, resetear, limpiar ni absorber este trabajo.

Los 22 commits desde `090a9bc2` hasta `dd3b181d` se publicaron en `origin/main-clean`; verificarlo de nuevo, no tratar este dato como garantía del estado futuro. Incluyen refactor por dominios, perfiles, canon embebido, escenas, herramientas, integración nativa y dos commits de geometría que deben inventariarse sin mezclarlos con correcciones de configuración.

Commits relevantes:

- `077cb32a`: perfil geometric versionado.
- `c67c11f3`: geometric como perfil canónico embebido.
- `59fa1965`: resolución de perfiles en la interfaz del mundo acoplado.
- `e1f337d3`: perfiles legacy 2030 y default 2026.
- `f0490c4d`: carga de manifiestos en el core.
- `881725be`: consumidores de scripts y resolución de perfiles.
- `09c3a834`: migración de escenas y lectores de telemetría/ruedas.
- `5613136a`: diagnósticos y checks de audio.
- `8e2baf48`: contrato de assets y herramientas Blender.
- `dd3b181d`: carga nativa versionada, paridad de fuente y serialización.

El fix posterior de séptima marcha está en el working tree y NO está commiteado ni publicado:

- `game/scripts/input/vehicle_rust_input_controller.gd`, modificado.
- `game/tests/test_vehicle_profile_manual_gears.gd`, nuevo.

Revisar ambos antes de continuarlos. No restaurarlos al HEAD. El archivo `instrucciones.md` también contiene trabajo local y este reemplazo documental no crea un commit.

## 2. Problema ya corregido localmente: séptima marcha

El perfil geometric conserva siete relaciones: `[3.45, 2.85, 2.40, 2.05, 1.80, 1.64, 1.50]`. El perfil legacy tiene seis. No hay que retocar relaciones, potencia ni ayudas para recuperar séptima.

El controlador leía `powertrain` directamente desde el manifiesto, no encontraba relaciones y conservaba un fallback de seis. El fix local obtiene la ruta del vehículo, resuelve los manifiestos con `PhysicalVehicleWorldInterface` y el loader Rust y deriva `maximum_forward_gear` de las relaciones. Conserva documentos planos y falla explícitamente si no puede cargar un perfil válido.

La prueba local envía eventos de entrada al controlador con un vehículo stub: confirma petición sexta → séptima para geometric versionado y plano, y bloqueo de séptima para legacy. No prueba por sí sola la conducción ni el cambio efectivo de la caja nativa. Auditar inicialización, selección de DLL Debug/Release, orden de escena y manejo de errores del fix. Añadir comprobación con las dos variantes reales y una caja de ocho relaciones en un fixture si la auditoría lo justifica; no codificar un límite fijo de siete.

## 3. Hallazgos iniciales pendientes de corrección

### H1. Definiciones de ambas variantes todavía apuntan al JSON plano

Archivos:

- `game/data/vehicles/f1_2030_v10.tres:11`.
- `game/data/vehicles/liveries/f1_2030_v10_mp4_6_senna_1.tres:11`.

Ambos conservan `res://data/vehicles/f1_2030/f1_2030_v10_geometric.json`, mientras los nodos de física/core y escenas de sesión usan `res://data/vehicles/f1_2030/profiles/geometric/vehicle.json`.

Los valores de combustible actualmente coinciden; no se comprobó una alteración del combustible físico del coche. El defecto es la coexistencia de fuentes: una edición futura de `fuel.json` puede cambiar la simulación sin actualizar el plan de sesión/pit stop.

Corregir los lectores H2 antes o en la misma unidad que estos recursos. No cambiar solamente las rutas: eso convierte los planes actuales en ceros. Comprobar también otras definiciones/variantes y referencias indirectas encontradas por la segunda auditoría.

### H2. Dos lectores de combustible no resuelven manifiestos

Archivos y funciones:

- `game/scripts/vehicle/vehicle_definition.gd:16`, `load_fuel_plan`.
- `game/scripts/runtime/lap_timing_controller.gd:271`, `_load_fuel_plan`.

Ambos buscan `parsed.fuel` en el documento principal. La reproducción con el manifiesto devuelve ceros para `capacity_kg`, `initial_kg`, `estimated_lap_consumption_kg` y `reference_lap_time_s`. El dominio activo declara 110 kg, 7.6 kg, 2.53 kg/vuelta y 90 segundos, respectivamente.

Impactos a verificar en ejecución:

- `pit_stop_controller.gd` pierde consumo estimado, mantiene el mínimo de vueltas seleccionables y calcula un objetivo de combustible cero. La capacidad puede recuperarse mediante snapshot nativo, lo que oculta parte del defecto.
- `LapTimingController` pierde el plan de vueltas y la referencia del delta de combustible.
- `game/tests/gdunit/lap_timing_controller_test.gd` ya usa el manifiesto versionado; esa suite no se ejecutó en la revisión inicial y no debe declararse verde.

En las sesiones actuales los lectores siguen recibiendo el perfil plano por H1, y por eso siguen obteniendo valores correctos. La simulación Rust carga correctamente el dominio fuel. No confundir un plan roto con la eliminación de combustible físico.

### H3. Telemetría de chasis conserva un lector plano

`game/scripts/telemetry/telemetry_manager.gd:559`, `_build_chassis_snapshot`, busca `parsed.chassis` en el manifiesto activo. Motor y transmisión ya tienen otro tratamiento.

`vehicle_mass` y `front_weight_distribution` no están registrados como propiedades Godot en `F194RustVehicle`, comprobado con `get_property_list`; el fallback de JSON era necesario y ahora deja campos nulos. El dominio chassis sí define ambos. Afecta el snapshot de configuración, no acredita un cambio de masa o reparto de la simulación.

Auditar todos los campos de ese snapshot y sus consumidores. Distinguir masa base declarada, masa operativa con combustible y reparto efectivo dinámico: no sustituir una por otra solo para eliminar nulls. El acceso a `front_left_wheel` es una limitación anterior documentada en `game/tests/AGENTS.md`; no atribuirla automáticamente a la migración.

### H4. Lectores de dominios suponen nombres fijos

`_read_profile_section` en `f1_wheel_visual_controller.gd` y `telemetry_manager.gd` construye `section.json` en lugar de respetar `domains[section]`. Los perfiles distribuidos usan esos nombres y no se demostró una pérdida actual de parámetros por este punto. Es una limitación del contrato para rutas alternativas.

Quien recibe debe decidir, con la arquitectura y pruebas del loader, cómo unificar la lectura. Preferir la configuración efectiva resuelta por Rust para parámetros físicos. Mantener separado el documento `suspension_presentation` utilizado por el renderizador. No crear otro resolvedor con reglas de rutas, defaults o digest distintas.

## 4. Segunda auditoría profunda obligatoria para el receptor

Ejecutarla antes de implementar las correcciones adicionales. Incluir tanto el rango completo `090a9bc2..dd3b181d` como el diff local del fix de marchas. Inspeccionar todos los archivos del rango, clasificando configuración, consumidores, geometría y evidencia; no editar cambios visuales por pertenecer al rango.

Construir una matriz por dominio y consumidor con: fuente anterior, ruta activa, resolución, parámetros utilizados, defaults/fallbacks, prueba existente y riesgo. Cubrir chassis e inventario de masas, fuel, steering, powertrain, suspension, tires, brakes, aerodynamics, driving_assistance, underfloor_contact, coupled_world, audio y suspension_presentation.

Rastrear lecturas directas de JSON, campos ausentes, constantes, límites de tamaños de arrays y fallbacks. Revisar:

- Loader Rust, conversión legacy, defaults por dominio y validaciones cruzadas.
- Perfil canónico embebido frente a archivos versionados y documentos planos.
- FFI, configuración de runtime, F194RustVehicle, F90Core, paridad BUILD/HEAD y serialización de enteros/precisión.
- Recursos `.tres`, escenas, controladores de entrada, combustible, vueltas, pit stop, telemetría, HUD, ayudas y visuales.
- Exportadores/validadores Blender, diagnósticos, audio, launchers y tests afectados.

No migrar archivos por coincidencia de nombres: HudConfig lee su propio documento de HUD; SuspensionGeometry lee geometría de presentación. Detectar referencias históricas deliberadas y fixtures legacy antes de cambiarlos.

Cada hallazgo debe tener disparador reproducible, parámetro esperado/observado, consumidor afectado, gravedad y criterio de aceptación. Separar regresiones presentes, incompatibilidades latentes y problemas preexistentes. La revisión inicial y pruebas de paridad no reemplazan esta auditoría.

## 5. Orden de resolución y criterios de aceptación

1. Inventariar estado y ejecutar la segunda auditoría. Entregar matriz y hallazgos confirmados; definir alcance antes de implementar según el protocolo del repo.
2. Revisar y consolidar el fix local de marchas, conservando la regresión.
3. Elegir una vía compartida de lectura de configuración efectiva. Corregir H2 y H3; resolver H4 sin mezclar configuración física y presentación.
4. Migrar H1 conjuntamente con sus consumidores. Verificar que todos usen el perfil seleccionado por la sesión, sin fuentes ocultas.
5. Añadir regresiones de comportamiento: planes no vacíos, consumo/delta de vueltas, selección y objetivo de refuel, telemetría correcta y relaciones accesibles.
6. Ejecutar validaciones, revisar el diff y separar commits por preocupación si hay autorización para commitear. No hacer push por recibir este archivo.

Aceptación mínima:

- Ambas variantes permiten séptima y un perfil legítimo de seis sigue limitado a seis.
- Capacidad, carga inicial, consumo estimado y referencia de vuelta coinciden con el perfil resuelto; ningún fallback a cero oculta un dominio ausente.
- Un fixture con cambio del consumo modifica coherentemente plan de vueltas y objetivo de refuel, sin tocar datos activos.
- Telemetría de chasis contiene los parámetros correctos y conserva el significado de cada masa/reparto.
- El manifiesto respeta sus rutas declaradas o rechaza explícitamente una ruta inválida; no usa silenciosamente un archivo hermano de nombre fijo.
- Paridad de configuración efectiva y digest preservada. No cambiar relaciones, valores físicos, goldens o tolerancias para ocultar fallos.

## 6. Validación y evidencia disponible

Evidencia previa en `scratch/vehicle_configuration_refactor/sprint6/`:

- `profile_migration_parameter_audit.md`: revisión inicial, no segunda auditoría.
- `profile_reader_omissions_reproduction.log` y `reproduce_profile_reader_omissions.gd`: lectores reales con ruta plana y manifiesto; propiedades nativas.
- `gear_fix_regression.log`: `VEHICLE_PROFILE_MANUAL_GEARS_PASS`.
- `gear_fix_smoke_original.log`: primer fallo en ratón/sliders del cockpit.
- `gear_fix_smoke_original_recheck.log`: recheck original PASS sin editar código.
- `gear_fix_smoke_senna.log`: Senna PASS.
- `implementation_result.md`, `installed_library_provenance.txt` y logs `committed_*`: validación anterior de la migración.

Comando del test nuevo:

```powershell
& .tools/godot/Godot_v4.7.1-stable_win64_console.exe --headless --path D:/Formula90s/game --script res://tests/test_vehicle_profile_manual_gears.gd
```

Validar suites GdUnit de lap timing, pit stop y telemetría relevantes mediante el runner del repo, verificando primero su interfaz. No sustituir aserciones de consumo por un smoke general.

Rust: `cargo check --manifest-path game/crates/Cargo.toml --workspace --all-targets`; suites `vehicle_profile_manifest_test`, `geometric_profile_parity_test`, `legacy_and_default_profile_parity_test`, `suspension_f1_2030_geometric_test` y `vehicle_profile_native_loading_test` del paquete `vehicle_physics_engine`. La validación anterior pasó 36 pruebas específicas Rust y tres Python. Eso no certifica todos los consumidores Godot.

Baselines generales previos: 14 fallos en física y tres en `facade_parity`; game_sim, vehicle_audio_engine y dsp-abi-check pasaron. Volver a identificar fallos actuales y compararlos, no asumir que cualquier fallo pertenece al baseline.

Usar `scripts/run_f1_94.ps1` para ValidateRuntimeOnly, Smoke, SmokeAudio, SmokePitStop y Senna conforme al alcance final. Los gates del mundo acoplado son independientes. Inspeccionar errores y carga efectiva del perfil además de exit/PASS. Smoke headless no certifica sonido percibido, geometría ni conducción sostenida.

Si se modifica Rust/nativo o se avanza HEAD, recompilar y verificar BUILD/HEAD y hashes instalados antes del runtime. No reutilizar DLL/caches entre ramas. La recompilación anterior encontró DLL mapeadas que impedían Copy-Item aunque el build devolvía éxito: verificar las copias reales; no confiar solo en exit 0. No editar build.rs ni falsear BUILD_SOURCE para saltar paridad.

## 7. Exclusiones y entrega

No remodelar ni exportar/promover el `.blend` o GLB para resolver este handoff. El candidato geométrico previo requiere revisión visual humana y queda independiente. No restaurar assets 2026 eliminados ni ejecutar herramientas históricas de restauración.

Preservar trabajo ajeno. Staging por rutas explícitas, nunca `git add -A`; revisar `git diff --cached`. No commitear DLL, BUILD_SOURCE, caches, imports o evidence generada. No adjudicar el fix de marchas local a los commits ya publicados.

Entregar: segunda auditoría con matriz completa, hallazgos confirmados y descartados, correcciones concretas, pruebas por hallazgo, fallos separados por origen, procedencia final y pendientes humanos. Mantener `plan sprint -> select backlog item -> implement -> review -> human approval -> done`; recibir este handoff no acredita aprobación final ni autoriza push.
