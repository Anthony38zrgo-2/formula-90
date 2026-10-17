# Handoff — Williams FW19 1997 Hockenheim / Jacques Villeneuve hasta Fuji

Fecha: 2026-10-17. Raíz: `D:/Formula90s`. Este documento reemplaza el handoff histórico P0/P1. Su copia íntegra está en `scratch/williams_publication_handoff_20261017/instrucciones_before_publication_handoff.md`. Actualización posterior autorizada: los dos blends actuales ya se copiaron y verificaron en car/source y driver/source de la variante. Los GLB, escenas, física y lanzamiento Williams siguen pendientes. El resto del plan conserva sus puertas de validación.

## 1. Objetivo y estado real

Publicar localmente la variante en `D:/Formula90s/game/assets/models/vehicles/f1-1997/williams/variants/hockenheim/villenueve`, conservando separados y editables el blend del coche y el del piloto, producir los GLB necesarios y conducir esa variante en Fuji con el launcher canónico y procedencia verificable. `villenueve` se mantiene exactamente como fue pedido y como existe en disco; la identidad es **Jacques Villeneuve**. No crear además una variante `villeneuve` por corrección silenciosa.

| Entrega | Estado confirmado |
|---|---|
| Extracción de dos paquetes y resolución del carset P0/P1 | Realizada; contenido privado, cobertura de temporada con límites |
| Williams Hockenheim texturizado y cuatro ruedas | Preparado y revisado en Blender |
| Carbono, volante F1 2030 y columna nueva | Guardados en fuente del coche |
| Piloto independiente con torso/piernas/arnés | Guardado, ocho objetos; casco/cuerpo rígidos, mangas y guantes con skin |
| Mangas y guantes azul/blanco/negro sin marcas | Atlas generados y empaquetados; postura revisada solo estáticamente |
| Nombres de objetos del piloto | Descriptivos; huesos originales conservados |
| Nombres y partición runtime del coche | PENDIENTES de inventario y normalización |
| GLB actuales del conjunto final | PENDIENTES; el GLB del depósito está obsoleto |
| Programa nuevo | AGENTS y dos blends fuente promocionados; hashes en reports/authoring_source_promotion.json; sin GLB nuevos |
| Perfil físico/escena/selector Williams | PENDIENTES |
| Importación y conducción Williams en Fuji | NO REALIZADAS |

Historia completa y reglas reutilizables: `game/assets/season/1997/AGENTS.md`. Detalle de cada ajuste: `COMPONENTES_PILOTO.md`. Resolución de texturas/traslado: `TEXTURAS_Y_ORGANIZACION.md`. No volver a ejecutar extracción o rehacer el aspecto aprobado sin una causa concreta.

## 2. Preservación de Git y de los artefactos

Rama observada: `main-clean`; HEAD `4f478b72977493656c2f2873e7fcee7ca21a1d62`. Antes de esta entrega ya estaban modificados THIRD_PARTY.md, instrucciones.md, game/BUILD_SOURCE y diez DLL bajo game/addons/formula90s/bin; además hay numerosos archivos sin seguimiento ajenos. Inventario completo previo en `scratch/williams_publication_handoff_20261017/repository_before.json`. Verificar de nuevo al retomar; no usar esta fotografía como garantía futura.

El adaptador existe en `scratch/grand_prix_four_checkout`, rama `codex/grand-prix-four-content-audit`, mismo HEAD en la comprobación. Tiene cambios sin publicar: módulos grand_prix_four y grand_prix_four_visual. Revisar `git status --short` sin omitir untracked y preservar también P2 antes de trasladar código. No borrar ese worktree ni copiar DLL/cachés entre ramas. Si se necesita un worktree de integración, inventariar y respaldar primero; no resetear el checkout principal.

Para cada fase: guardar rama/HEAD/estado y hashes iniciales, trabajar en scratch, producir evidencia, revisar cambios y alcanzar su puerta de aceptación. Stage únicamente rutas explícitas si se pide commit; nunca git add -A. No incluir archivos ajenos, DLL ni BUILD_SOURCE generados. No publicar medios de terceros porque exista un AGENTS nuevo.

## 3. Fuentes y evidencias que se deben abrir

Todas las rutas siguientes son relativas a `D:/Formula90s/`:

| Ruta | Uso |
|---|---|
| game/assets/season/1997/williams_completed/williams_fw19_textured.blend | Fuente actual del coche sin piloto; conservar |
| game/assets/season/1997/williams_completed/jacques_villeneuve_williams_fw19_1997_driver.blend | Fuente actual del piloto con atlas generados; conservar |
| game/assets/season/1997/williams_completed/textures/ | Atlas nuevos y texturas disponibles; verificar referencias realmente usadas |
| game/assets/season/1997/extracted/combined/ | Carset combinado, fuentes originales de solo lectura |
| game/assets/season/1997/extracted/inventory.csv y source_packages.json | Integridad de paquetes y archivos |
| game/assets/season/1997/conversion/reports/season_manifest_current.json | Resolución actual del carset y raíz vigente |
| game/assets/season/1997/conversion/reports/p2_execution_report.md | Evidencia de conversión anterior, no validación final actual |
| scratch/grand_prix_four_checkout/tools/importers/grand_prix_four/ | Resolver, auditoría e inventario P0/P1 |
| scratch/grand_prix_four_checkout/tools/importers/grand_prix_four_visual/ | cli.py, conversion.py, texture_decoder.py y blender_convert.py |
| scratch/williams_steering_mockup/ | Respaldos y reportes de montaje, giro del piloto, separación y hombros |
| scratch/williams_sleeve_image_generation/ | Prompts exactos, atlas objetivo/referencia, generation_report.json y respaldo previo |
| game/assets/season/1997/COMPONENTES_PILOTO.md | Estado vigente primero, historial después |
| THIRD_PARTY.md | Créditos y limitaciones de redistribución |

## 4. Estructura canónica de todos los modelos de 1997

Usar `f1-1997/<equipo>/variants/<circuito>/<piloto>/`. La primera instancia es:

```text
game/assets/models/vehicles/f1-1997/AGENTS.md
  williams/AGENTS.md
    variants/AGENTS.md
      hockenheim/AGENTS.md
        villenueve/AGENTS.md
          manifest.json                                      [PENDIENTE]
          car/AGENTS.md
            source/AGENTS.md
              .gdignore
              williams_fw19_hockenheim_1997_car.blend          [COPIADO Y VERIFICADO]
              textures/AGENTS.md                              [mapas fuente PENDIENTES]
            runtime/AGENTS.md
              williams_fw19_hockenheim_1997_chassis.glb        [PENDIENTE]
              williams_fw19_hockenheim_1997_wheel_front_left.glb
              williams_fw19_hockenheim_1997_wheel_front_right.glb
              williams_fw19_hockenheim_1997_wheel_rear_left.glb
              williams_fw19_hockenheim_1997_wheel_rear_right.glb
          driver/AGENTS.md
            source/AGENTS.md
              .gdignore
              jacques_villeneuve_williams_fw19_1997_driver.blend [COPIADO Y VERIFICADO]
              textures/AGENTS.md                                [atlas fuente PENDIENTES]
            runtime/AGENTS.md
              jacques_villeneuve_williams_fw19_1997_driver.glb   [PENDIENTE]
          reports/AGENTS.md
            source_inventory.json, object_name_mapping.json     [PENDIENTES]
            coordinate_and_pivot_report.json                    [PENDIENTE]
            export_validation.json, godot_import_validation.json
            fuji_validation.json                                [PENDIENTE]
```

Los nombres largos y completos de ficheros nuevos evitan abreviaturas nuevas. Los identificadores GEO_* y nodos SpinVisual/BrakeStatic ya son contratos del proyecto. Hay AGENTS en todos estos directorios por petición explícita, también en hojas de medios, excepción a la regla general de hojas binarias. Source/.gdignore impide que Godot importe blends y fuentes; runtime debe permanecer importable. `reports/` guarda manifiestos/evidencia compacta; capturas grandes, logs completos y temporales van en scratch. Este árbol es una estructura preparada, no prueba de que sus archivos pendientes existan.

Fuentes específicas de cada variante viven bajo ese piloto hasta demostrar igualdad por hashes con otra variante. Compartir recursos comprobados mediante referencia, no duplicar automáticamente ni crear carpetas common sin una necesidad concreta. Conservar originales en el depósito privado y promocionar copias verificadas; no mover ni vaciar extracted/.

## 5. Referencias de implementación que hay que leer

| Dominio | Archivos reales de referencia; inspeccionar de nuevo antes de editar |
|---|---|
| Nombres y exportación | tools/blender/formula_one_2030_object_names.py, export_f1_2030_v10.py, validate_f1_2030_v10_glbs.py |
| Programa y manifiesto | game/assets/models/vehicles/f1-2030/manifest.json y AGENTS.md |
| Escena de vehículo | game/scenes/vehicles/f1_2030_v10/f1_2030_v10_rust.tscn |
| Ruedas | game/scripts/vehicle/f1_wheel_visual_controller.gd |
| Volante y piloto | game/scripts/vehicle/steering_wheel_visual_controller.gd, driver_visual_controller.gd, driver_arm_inverse_kinematics_modifier.gd |
| Contratos del donante | game/assets/models/drivers/driver_manifest.json, AGENTS.md y source/prepared_driver.blend |
| Validación del driver actual | tools/drivers/validate_driver_geometry_contract.ps1 |
| Perfil versionado | game/data/vehicles/f1_2030/profiles/geometric/vehicle.json y sus domains |
| Definición registrable | game/data/vehicles/f1_2030_v10.tres, game/scripts/vehicle/vehicle_definition.gd |
| Sesión Fuji | game/data/race_sessions/f1_2030_v10_fuji76_77.tres, game/data/tracks/fuji76_77.tres |
| Sesión runtime | game/scenes/runtime/vehicle_test_session.tscn y world_hud_compositor.tscn |
| Launcher y BUILD | scripts/run_f1_94.ps1, game/BUILD_SOURCE; leer validaciones antes de ampliarlas |
| Smoke/conducción | game/tests/smoke_test_vehicle_audio.gd, smoke_test_aerodynamics_backends.gd, test_simplified_autonomous_driver_fuji.gd; comprobar límites de parametrización |

No ejecutar exportadores de F1 2030 directamente sobre Williams suponiendo que son genéricos. Reutilizar funciones/contratos comprobados mediante un adaptador que acepte raíces de entrada/salida explícitas. La referencia F1 2030 contiene neumáticos, frenos y rig con contratos específicos; no copiar sus valores físicos ni sus SHA al manifiesto nuevo.

## 6. Plan secuencial pendiente y puertas de aceptación

### Paso 1 — Congelar el punto de partida y seleccionar backlog

Inventariar ambas fuentes y sus imágenes, objetos, materiales, UV, matrices, jerarquías, poses y modificadores con SHA-256. Guardar snapshot de coche/piloto y entradas de configuración en un directorio nuevo de scratch. Recuperar el reporte de generación y prompts de atlas. Revisar créditos reales por pieza. Confirmar el item del sprint: publicación visual Williams, seguida de integración/prueba, sin alterar otros programas. Esta entrega no ejecuta ese item.

Aceptación: source_inventory.json identifica exactamente dos blends actuales y hashes de recursos usados; no referencia GLB viejo como fuente ni considera un respaldo como versión activa. Las fuentes originales y el checkout ajeno están preservados.

### Paso 2 — Renombrar y clasificar piezas del coche

Abrir copia del coche en Blender y listar todas sus mallas. Resolver cada pieza por geometría/material/posición, no por semejanza del nombre. Guardar object_name_mapping.json con nombre original/final, función, grupo de exportación y fingerprint. Conservar mallas y UV; no fusionar ni remallar para lograr la convención.

| Pieza actual | Nombre/contrato objetivo |
|---|---|
| Williams visible steering column | GEO_STEERING_COLUMN_SHAFT |
| GEO_STEERING_WHEEL_BODY, DISPLAY, CENTER_FASTENER | Conservar nombres contractuales ya existentes |
| GEO_STEERING_WHEEL_PADDLE_LEFT/RIGHT | Conservar, pero verificar lado real después del giro del conjunto |
| Williams donor steering wheel adjustment | Control de montaje descriptivo; crear/exportar pivote sin incluir auxiliares inútiles |
| cockpit y nose_cone | Clasificar carrocería/interior real antes de mapear; no renombrar todo cockpit como asiento |
| left/right_*suspension | GEO_SUSPENSION_FRONT_LINK_ASSEMBLY / REAR_LINK_ASSEMBLY o componentes explícitos según inventario |
| wheel_front_left_* y otras tres esquinas | GEO_WHEEL_FRONT_LEFT_TIRE, RIM_AND_SPOKES, HUB, CENTER_LOCK_NUT y equivalentes por esquina, solo si la geometría existe |
| Espejos, alerones, suelo, escapes | Usar familias GEO_COCKPIT_*, GEO_FRONT_WING_*, GEO_REAR_WING_*, GEO_UNDERBODY_* y GEO_EXHAUST_* verificadas |

No asignar disco/pinza a una malla por obligación de una plantilla. Mantener frenos existentes identificados. SpinVisual rota neumático/rin/buje/disco según contrato vigente; BrakeStatic mantiene pinza/conductos sin rotar. Las acciones de boxes que sustituyen solo rin/neumático no deben arrastrar el disco; son contratos distintos. Registrar ausencia de piezas en vez de crear duplicados ficticios.

Aceptación: ningún consumidor depende de nombres viejos sin migrar; no hay duplicados ambiguos ni sufijos accidentales .001; fingerprints de geometría/UV coinciden con la fuente salvo cambios expresamente aprobados. Reabrir el blend normalizado y verificar materiales.

### Paso 3 — Coordenadas, ruedas, pivotes y separación

Medir escala real del Williams, centros/radios de ruedas, batalla, vías, suelo y distancia al origen. Fuente actual X lateral, Y delante, Z arriba; runtime +X derecha, +Y arriba, -Z delante, metros. La conversión candidata es `(x,y,z)->(x,z,-y)`, pero verificar signos y handedness con morro, lado izquierdo y piloto antes de aprobarla. Aplicar la misma matriz a coche y piloto en copias de exportación y guardar inversa y unidades. No aplicar la conversión estándar de glTF una segunda vez.

Colocar pivotes de ruedas en sus centros, con giro local X; llevar las mallas de cada GLB de rueda a su origen local y conservar los offsets en manifest/escena. Pivote del volante en el eje de columna, sin deriva al girar. El montaje manual actual del control del volante está en `(0,0.3225931227,-0.015)` m; columna desde `(0,0.3345931172,-0.015)` hasta `(0,0.4620317221,-0.015)`, longitud ~0.127439 m. Verificar de nuevo contra el blend, no imponer estas cifras si el usuario volvió a editarlo. La columna termina en el elemento blanco de la textura bajo el tablero.

El cuerpo original se separó por material fw19_d (1700 caras) y debe seguir fuera del GLB de chasis; casco y brazos solo en piloto. No duplicar piloto al ensamblar. Sus ocho objetos conservan world coordinates de montaje; no centrarlo arbitrariamente sin registrar el nuevo offset. Los hombros se ajustaron a las zonas blancas sin mover las manos. El giro de raíz de 180° ya está incorporado en su fuente: no repetirlo por copiar sin revisar driver_visual_controller.

Aceptación: coordinate_and_pivot_report.json contiene matrices comunes, medidas del Williams y offsets de cada rueda/piloto; reensamblar ambos blends a identidad reproduce el montaje; los pivotes funcionan en pruebas de giro/rotación sin traslación lateral accidental.

### Paso 4 — Contrato del piloto y revisión de agarre

Este piloto es híbrido: casco y cuerpo originales GP4 rígidos, brazos/guantes F1 2030 con skin y huesos conservados. No contiene DriverOriginalUniformGeometryContract y DriverOriginalGloveGripContract completos como el driver canónico ni cumple su garantía de anatomía original íntegra. El controlador actual y el validador del launcher rechazan fuentes distintas: no añadir metadatos falsos, no sustituir driver.glb global ni desactivar esa validación.

Primera integración visual: exportar piloto independiente con su postura estática verificada y declararla estática en el manifest/escena. Si se requiere giro con manos acompañando y cámara ocular, diseñar un contrato propio versionado para este híbrido, separar bind/rest pose de la pose inicial, conservar los huesos y resolver skin del torso/cuello o partes rígidas a hueso antes de afirmar que está animado. Crear validación específica sin debilitar la del F1 2030. Elegir este alcance antes de esa implementación; no fingir compatibilidad por coincidir nombres de huesos.

Revisar agarre con volante ensamblado: palmas en laterales, pulgares cara del piloto, dedos detrás hacia levas, muñecas sin inversión y hombros sin huecos visibles. La postura Blender es candidata y recibió varias correcciones; no usar matrices de intentos fallidos como golden. Si se activa animación, validar centro y recorrido completo autorizado con dedos, muñecas, codos y contactos; limitar el primer test a estático si ese contrato no está listo y declararlo en la evidencia.

Aceptación: informe del piloto distingue piezas rígidas y deformables, soporta carga del GLB sin doble rotación ni skin roto y especifica qué movimientos han sido validados. La aprobación visual del usuario sigue siendo necesaria.

### Paso 5 — Materiales y exportación de seis GLB

Conservar el azul del torso, los atlas generados sin logos en brazos/guantes y los mapas de carrocería Hockenheim. Usar las imágenes generadas conectadas directamente, no los viejos materiales con Ferrari/44. Preservar el carbono lineal `(0.018,0.018,0.018,1)`, roughness .64, metallic 0 y opacidad; alfa fuente representa detalle especular, no transparencia. El rango .16–.24 especular puede necesitar bake/extensión o una equivalencia compatible documentada. Comprobar visor oscuro de alpha cero según material real, no volverlo invisible automáticamente. No añadir mapas de entorno inexistentes.

Preparar exportador parametrizado en tools/blender o adaptar el código P2 con source-file/output-directory explícitos; nombres nuevos completos, sin comentarios de código. Exportar chasis+volante+columna, cuatro ruedas y piloto: seis GLB. Embebidos los recursos usados, sin cámaras/luces de estudio ni controles sin función. Las ruedas deben contener únicamente su esquina y grupos correspondientes; chasis excluye ruedas/piloto; piloto excluye coche/volante.

Reimportar cada GLB en Blender limpio: materiales, alfa, normales, UV, imágenes, orientación, dimensiones, skin y pose. Comparar triángulos/posiciones/corners con copias normalizadas; validar valores finitos, joints válidos, pesos normalizados y ausencia de imágenes externas perdidas. Verificar que exportación no aplicó dos veces transformaciones de armature. Para GLB simples no basta un FINISHED del operador.

Aceptación: seis archivos reproducibles, hashes/tamaños, registro de la versión de Blender/exportador y export_validation.json sin fallos. Fotografías antes/después y prueba de reensamblado en scratch. El GLB histórico no participa en la publicación nueva.

### Paso 6 — Promoción local de fuentes, texturas, GLB y manifest

Tras la revisión del candidato, usar tools/common/output_policy.py para cada destino. Copiar ambos blends actuales a car/source y driver/source con los nombres del árbol; incluir texturas originales realmente necesarias y las generadas en sus source/textures, empaquetadas además en el blend. Mantener originales privados y respaldos. Promocionar los seis GLB desde scratch a los runtime. Volver a abrir los blends desde el destino para detectar rutas rotas.

Crear manifest.json con identidad, slug de carpeta, circuito, piloto, procedencia por pieza (original/derivada/generada/donante), créditos/derechos, hashes de dos blends y texturas, hashes/tamaños/rutas relativas de los seis GLB, contrato de coordenadas, offsets de ruedas/driver, nombres contractuales y estado real de validación. Basarse en el schema actual sin asumir que admite el driver: ampliar explícitamente los consumidores si hace falta. Registrar perfil físico y su digest resuelto; no copiar el digest o vehicle_id del F1 2030. Separar visual_ready, import_ready, runtime_ready y fuji_review con evidencia, no estados aprobados ficticios.

Aceptación: ningún runtime depende de scratch, rutas del usuario o generated_images; todas las referencias existen dentro del programa/recursos compartidos autorizados; hashes coinciden. Publicación local no autoriza subir el mod ni la fuente a un remoto público. Conservar créditos y verificar permisos antes de distribución externa.

### Paso 7 — Perfil físico y definición independiente

Antes de conducir decidir con el usuario si el objetivo es prueba visual con física donante explícita o un perfil Williams 1997 interpretado del mod. No inferir masa, aero, neumáticos, marchas, potencia o suspensión desde un GLB ni vender el F1 2030 como física FW19. El trabajo documental no autoriza calibración del motor Rust/C++.

Rutas propuestas, aún no implementadas: `game/data/vehicles/f1_1997/williams/profiles/hockenheim/vehicle.json` con domains separados, `game/data/vehicles/f1_1997_williams_hockenheim_villenueve.tres`, `game/data/cameras/f1_1997_williams_hockenheim_villenueve_cockpit.json`. Crear AGENTS en cualquier carpeta nueva de esos dominios antes de editar código/configuración.

Resolver cada dato como extracted/derived/estimated; desconocidos explícitos, unidades SI y referencias. Crear configuración de suspensión visual y puntos de rueda medidos sin alterar la dinámica para ocultar defectos del modelo. Reusar el resolver de perfil versionado y el digest vehicle_profile_provenance_v1. Si se aprueba física donante para ensayo visual, registrar su origen y etiquetas de prueba en manifest/telemetría; no modificar el donante global.

Aceptación: resolución de todos los domains, unidades y digest válida, NodePaths coherentes con la escena, parámetros medidos frente a mallas y denominación honesta del alcance físico. Datos sin resolver bloquean la afirmación de simulación Williams.

### Paso 8 — Escenas, cámara, audio y sesión Fuji dedicadas

Rutas propuestas: `game/scenes/vehicles/f1_1997_williams/f1_1997_williams_hockenheim_villenueve.tscn`, `game/data/race_sessions/f1_1997_williams_hockenheim_villenueve_fuji76_77.tres` y `game/scenes/runtime/williams_hockenheim_villenueve_fuji_test_session.tscn`. Reusar el compositor y Fuji existentes; selected_vehicle debe resolver el Williams nuevo y selected_track `res://data/tracks/fuji76_77.tres`. No reemplazar la sesión F1 2030 predeterminada ni cambiar geometría del circuito.

La escena arma chasis, cuatro esquinas y driver con offsets del manifest. Mantener jerarquía Hub/SteerPivot/CamberPivot/SpinVisual y BrakeStatic, evitar doble autoridad de transforms. Enlazar volante/columna y levas correctas, cámara cockpit sin atravesar casco o tablero, cámara externa para revisión, HUD/input y audio existentes con rutas declaradas. No copiar configuración del asiento/cámara F1 2030 como si coincidiera con FW19. El controlador de mundo y presentación/audio debe apuntar al mismo perfil Williams que VehicleRigidBody.

Aceptación: escena instancia todos los recursos sin duplicados, cámara usable, controles reconocidos, ruedas en sus centros y perfil coincidente en física/presentación/audio; piloto estático o animado según contrato realmente aprobado. Importar y revisar con Godot MCP, no solo inspección textual.

### Paso 9 — Ampliar el launcher canónico sin saltar validaciones

scripts/run_f1_94.ps1 hoy solo acepta liveries original/mp4_6_senna_1 y exige manifest/asset del F1 2030; valida además el driver global. Por tanto NO existe hoy comando válido para lanzar este Williams. No usar -Livery original y cambiar archivos debajo, ni lanzar directamente la escena con godot_run/run_project.

Añadir un selector de vehículo/variante explícito con defaults anteriores intactos; el nombre exacto del parámetro y sus pruebas son una tarea pendiente, no una interfaz existente. Resolver escena, vehicle_id, manifest, perfil, validación de driver y restricciones de modos desde ese selector. Cada modo smoke debe recibir escena/id correctos. Los validadores canónicos F1 2030 deben seguir pasando para su programa. La variante Williams debe tener su contrato propio y fallar explícitamente si faltan assets o perfil; no fingir ser F1_2030_V10.

Conservar BUILD/HEAD, procedencia de DLL y validaciones de hashes/perfil. Si cambios nativos requieren full rebuild, verificar HEAD, limpiar Cargo/SCons/game/.godot en el checkout de build adecuado después de preservar cambios; nunca reutilizar cachés de otra rama. godot_build solo importa/verifica recursos, no recompila las bibliotecas Rust/C++. Documentar el comando concreto recién implementado y validado en reports antes de la prueba; no inventar flags en este handoff.

Aceptación: selector nuevo elige inequívocamente Williams, ruta y hashes; modos original y otra livery sin regresión; launcher rechaza BUILD incorrecto y manifest ausente; no cambia silenciosamente la selección del usuario.

### Paso 10 — Importación Godot y smoke del Williams

Usar Fulviuus/godot-mcp, comenzar con godot_project_info y usar recursos/escenas/logs correspondientes. Importar en un entorno aislado o controlado y revisar los seis GLB, texturas embebidas, materiales Compatibility, skeleton y transforms. Los bridges nativo y avanzado tienen requisitos distintos: comprobar sesión; reportar limitación si no existe, sin afirmar captura/runtime a partir de texto. No instalar bridges ni cambiar autoloads sin inventariar esos cambios; dejarlos desactivados en uso normal.

Crear/adaptar smoke parametrizado para la variante: carga del manifest, perfil, seis assets, escena/driver y cámaras, sin errores de importación ni NodePaths inválidos. Usar el nuevo selector del launcher para los smoke. Si un test Fuji existente está ligado al F1 2030, parametrizarlo y mantener su regresión; no interpretar su PASS como cobertura Williams.

Aceptación: godot_import_validation.json con versión, renderer, errores/cero pendientes y recursos cargados; smoke específico identifica vehicle_id y perfil Williams o prueba visual donante declarada. Guardar logs y comando real con HEAD/BUILD y hashes de assets.

### Paso 11 — Prueba real en Fuji y revisión humana

Lanzar mediante el comando canónico ya extendido, y confirmar en logs/manifest la variante Hockenheim/Villenueve realmente cargada, pista Fuji76_77 y tipo de perfil. Comenzar en parado: escala frente a pista, contacto de cuatro ruedas, separación de frenos, altura de suelo, texturas, carbono y ausencia de torso duplicado. Capturar frente, trasera, ambos laterales, cockpit y unión de hombros/manos con volante. Conducir salida, frenada, marchas, recta y curvas a ambos lados, al menos una vuelta completa sin caída, vuelco espurio, piezas desancladas ni errores de runtime. Registrar tiempo de observación/vueltas, no inventar telemetría de rendimiento.

Verificar suspensión y dirección visual bajo carga, giro/spin independiente, cámara, HUD y audio sin regresiones ajenas. Para piloto estático declarar que las manos no siguen el giro; para contrato animado aprobado comprobar su recorrido, pulgares al frente y dedos hacia levas, sin inversiones ni colapso de muñecas. No cambiar la física para tapar clipping. Si una condición impide terminar, registrar defecto y fase bloqueada en vez de marcar aprobado.

fuji_validation.json debe incluir commit/dirty snapshot y BUILD, comando, vehículo/circuito/perfil, hashes, versión de engine, resultado de smoke, capturas/logs/telemetría, vuelta(s) real(es), defectos y aprobación visual/conducción del usuario. Solo entonces marcar prueba Fuji completa. Aprobación de extracción, Blender o docs no sustituye esta revisión.

### Paso 12 — Cierre y repetición del pipeline

Actualizar manifest, AGENTS e instrucciones con archivos realmente publicados, comandos probados y defectos pendientes. Si se solicita commit, separar docs/herramientas propias de medios de terceros, revisar git diff --cached y no subir licencias/recursos no autorizados. La ruta local de publicación no implica despliegue público. Conservar fuentes, reportes y hashes para regenerar.

Segunda validación propuesta: Ferrari F310B Hockenheim con la misma estructura y puertas. Resolver sus recursos/datos de forma independiente; solo después ampliar a otros equipos/circuitos/pilotos. El Williams es modelo de proceso y contratos, no una plantilla de valores físicos ni de selección de texturas universal.

## 7. Checklist de siguiente agente

- Leer root AGENTS, protocolo de season/1997 y AGENTS del destino; verificar Git en ambas copias y backups.
- Abrir los DOS blends actuales y generation_report.json; no usar el GLB histórico.
- Ejecutar primero pasos 1–3: inventario, mapa de nombres, medidas/pivotes y coordenadas; revisar antes de exportar.
- Resolver alcance estático/animado del piloto y prueba física antes de tareas que dependan de ello.
- Exportar/reimportar seis GLB en scratch y pasar validaciones, luego promoción de fuentes y assets.
- Crear perfil/definición/escenas y selector real del launcher, sin alterar F1 2030 ni el motor por conveniencia.
- Importar y smoke con Godot MCP; lanzar canónicamente y probar el Williams en Fuji con evidencia y revisión humana.
- No dar por terminada ninguna puerta pendiente en esta lista por el hecho de que exista su directorio.
