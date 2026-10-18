# Backlog priorizado — Williams 1997: integración y físicas simplificadas

Fecha: 2026-10-18. Repositorio: D:/Formula90s.
Rama verificada antes de editar: main-clean.
HEAD verificado: d3bbe0a312cb7d747cf72b8a97ebd7492cc84bb9.
Estado: análisis realizado; SIMPLIFIED-01 a 05 cerrados y aprobados; SIMPLIFIED-06 implementado, pendiente de aprobación. Este documento reemplaza íntegramente las instrucciones anteriores.

## Alcance obligatorio

Sólo se permite cambiar el comportamiento de las físicas simplificadas. Standard debe conservar sus ecuaciones, resultados, contactos, propiedades de masa, perfiles y comportamiento de ejecución actuales. El mundo físico acoplado independiente tampoco forma parte del alcance.

No equiparar automáticamente aerodinámica simplificada con suspensión simplificada: son selectores distintos. Verificar qué backend controla la entidad y propagar una selección explícita al puente, al resolver de masas y al fondo cuando sea necesario. No seleccionar una corrección por nombre de coche ni por la existencia de geometría: Standard puede usar el mismo Williams y la misma geometría.

En archivos compartidos, crear rutas específicas o seleccionar por backend antes de cambiar fórmulas o datos. Conservar la ruta Standard existente. No modificar globalmente el perfil JSON ni la escena compartida para aplicar correcciones exclusivas de Simplified. Si hacen falta datos distintos, usar una configuración o variante exclusiva de esa ruta. Los contratos compartidos sólo podrán ampliarse de forma compatible, actualizando conjuntamente emisor y receptor; no cambiar silenciosamente el significado de un campo utilizado por Standard.

La autorización de este encargo es documental: no implementar, reconstruir ni crear commits al redactar este backlog. Para la futura ejecución, seguir plan sprint → seleccionar ítem → implementar → revisar → aprobación humana → cerrar.

## Procedencia, evidencia y límites

El árbol ya estaba sucio: BUILD_SOURCE, binarios generados, instrucciones.md y cambios ajenos en audio, visuales de ruedas, pruebas y scripts. Preservar todo trabajo preexistente; no cambiar de rama ni resetearlo. Repetir inventario y comprobar HEAD antes de implementar. Mantener commits atómicos y staging por rutas explícitas.

Evidencia de esta auditoría:
- D:/Formula90s/scratch/williams_bridge_physics_audit_20261018/diagnostico.md
- D:/Formula90s/scratch/williams_bridge_physics_audit_20261018/mass_properties_by_fuel.json
- D:/Formula90s/scratch/williams_bridge_physics_audit_20261018/status_at_analysis.txt

Hay seis hallazgos confirmados por código, geometría, contrato de API o consulta de la DLL. Sus efectos dinámicos son posibles consecuencias; no se ha demostrado que cada uno cause el serpenteo o el vuelco. Godot MCP permitió inspeccionar la escena, pero la consulta de ejecución devolvió no_game. No hay aceptación dinámica nueva.

No repetir ni revertir como parte de este backlog las correcciones de contacto multirrueda y conservación de momentos de 670919f2, ni la integración vertical implícita y transferencia de impulsos simplificada de d3bbe0a3. Standard fue conservado deliberadamente en su integración anterior.

## Resumen priorizado

| Orden | Ítem | Prioridad | Hallazgo confirmado | Posible efecto | Solución propuesta | Dependencias |
|---|---|---|---|---|---|---|
| 1 | SIMPLIFIED-01 | Alta | Desplazamiento local del buje sumado a un contacto mundial | Palancas y velocidades dependientes de la orientación; guiñada con poco balanceo | Transformar el desplazamiento al marco mundial en la ruta simplificada | Ninguna |
| 2 | SIMPLIFIED-02 | Alta | Masa viva de Core combinada con centro e inercia de otra instancia | Desacuerdo entre fuerzas de Rust y respuesta angular de Godot al variar combustible | Publicar propiedades del mismo estado de Core y sincronizarlas coherentemente | Ninguna; coordinar con 03 |
| 3 | SIMPLIFIED-03 | Alta | Centro compuesto sustituido sin trasladar su tensor de inercia | Altura y transferencia incompatibles con inventario y combustible | Resolver centro e inercia respecto al mismo punto, sólo para Simplified | Ninguna; completar antes de cerrar 02 |
| 4 | SIMPLIFIED-04 | Alta | Contactos rígidos mundiales enviados como locales | Detección y velocidad de contacto dependientes de la ubicación del circuito | Convertir posiciones y usar velocidad del punto con contrato explícito | 02 para comprobación final |
| 5 | SIMPLIFIED-05 | Alta | Momento y velocidad del fondo referidos al origen | Cabeceo y amortiguación incorrectos al activar el fondo | Calcular ambos respecto al centro de masa mundial de la entidad | 02 y 03 |
| 6 | SIMPLIFIED-06 | Media; obligatoria antes de validar fondo | Sondas y bases de colisión por encima del centro de masa publicado | Fondo sin protección coherente mientras el coche está vertical | Medir el modelo y configurar sondas y colisión exclusivas de Simplified | 02, 03, 04 y 05 |

Las prioridades consideran relación con el síntoma y seguridad de integración. 04 y 05 deben corregirse antes de bajar sondas o activar nuevas colisiones mediante 06.

## Estado de ejecución

- SIMPLIFIED-01 — cerrado y aprobado el 2026-10-18. El contrato de marco vive en `game/crates/vehicle-physics-engine/src/simplified_suspension.rs`; la orientación se propaga en `suspension.rs` y `simulation.rs`; la prueba de invariancia 0/90/180° está en `game/crates/vehicle-physics-engine/tests/simplified_contact_frame_invariance_test.rs`. Standard conserva su referencia anterior. Evidencia: `scratch/simplified_01_hub_contact_frame_20261018/reporte_simplified_01.md`. Sin rebuild de binarios ni validación de conducción en este cierre.
- SIMPLIFIED-02 — cerrado y aprobado el 2026-10-18. Core publica masa, centro e inercia resueltos antes del solve en un bloque apéndice ABI 20 que sólo se activa con el backend Simplified; el puente aplica los tres al cuerpo y conserva la ruta anterior para Standard. Evidencia: `scratch/simplified_02_authoritative_body_properties_20261018/reporte_simplified_02.md`. Sin binarios instalados ni validación en Godot.
- SIMPLIFIED-03 — cerrado y aprobado el 2026-10-18. Simplified usa el centro compuesto completo con su tensor referido a ese mismo centro y una calibración coherente de las alturas del cuerpo para que, en la carga de referencia (`fuel.initial_kg`), el centro compuesto caiga exactamente en `center_of_gravity_height_offset`; a 0/7.6/105 kg el centro queda en −0.245455 / −0.243650 / −0.223944 m y el combustible sigue moviéndolo. Standard conserva −0.243650 m y sus resultados. La compuerta Williams vuelve a pasar a 120/240/480 Hz (7.61/9.71/10.53°). Evidencia: `scratch/simplified_03_mass_reference_20261018/reporte_simplified_03.md`. Sin binarios instalados ni validación en Godot.
- SIMPLIFIED-04 — cerrado y aprobado el 2026-10-18. El puente envía, en un bloque apéndice ABI 21, la posición en el marco del cuerpo, la normal mundial y la velocidad tangencial relativa calculada con las velocidades de Godot; Simplified clasifica con el punto del cuerpo, mientras Standard conserva sus campos y su resultado. Evidencia: `scratch/simplified_04_contact_contract_20261018/reporte_simplified_04.md`. Sin binarios instalados ni validación en Godot.
- SIMPLIFIED-05 — cerrado y aprobado el 2026-10-18. El paso del fondo recibe el centro de masa mundial autoritativo en Simplified y calcula la palanca, la velocidad del punto y el momento respecto a él (`r × F`); Standard conserva la palanca desde el origen. Evidencia: `scratch/simplified_05_underfloor_centre_20261018/reporte_simplified_05.md`. Sin binarios instalados ni validación en Godot.
- SIMPLIFIED-06 — implementado el 2026-10-18; pendiente de aprobación humana y del rebuild coordinado. El C++ mide el fondo real del chasis en el marco del cuerpo (−0.513751 m, igual al AABB local del GLB), coloca las cinco sondas a esa altura y añade una colisión de fondo exclusiva de Simplified; Standard no cambia. Validado en vivo por el puente: holgura de reposo 0.062–0.068 m, sin activación, y compresión progresiva con el muelle entrando por debajo de 0.012 m (fuerzas 4.6–7.6 kN en el difusor). Evidencia: `scratch/simplified_06_underfloor_geometry_20261018/informe_06.md`. El puente (`game/addons/godot_mcp` + autoload `MCPBridge`) sigue instalado, inerte sin `GODOT_MCP_BRIDGE_PORT`.

## SIMPLIFIED-01 — Contacto del buje en un único marco

Confirmado: suspension.rs suma hub_lateral_m y hub_long_m al punto mundial obtenido de los rayos. El desplazamiento pertenece al marco local del vehículo y procede del paso anterior.

Posible solución: pasar la orientación del cuerpo a la ruta simplificada que construye el contacto y rotar el desplazamiento antes de sumarlo. Definir expresamente el instante de muestreo; revisar el desfase del buje sin inventar una segunda integración. Evitar transformar dos veces el mismo contacto. No cambiar sample_contact para Standard de manera incondicional.

Archivos candidatos a modificar:
- D:/Formula90s/game/crates/vehicle-physics-engine/src/suspension.rs — muestreo y selección por SuspensionBackend.
- D:/Formula90s/game/crates/vehicle-physics-engine/src/simplified_suspension.rs — preparación y datos propios del backend.
- D:/Formula90s/game/crates/vehicle-physics-engine/src/simulation.rs — entrega de orientación y consumo del contacto.
- D:/Formula90s/game/crates/vehicle-physics-engine/tests/williams_vertical_integration_test.rs — ampliar regresión pertinente o crear una prueba específica de invariancia en ese directorio.

Aceptación: al rotar conjuntamente coche, carretera, contactos y velocidades, las fuerzas y momentos simplificados deben rotar de igual manera. Probar 0, 90 y 180 grados, incluyendo recorrido asimétrico trasero. Standard debe reproducir su referencia anterior.

## SIMPLIFIED-02 — Propiedades de masa desde el estado autoritativo

Confirmado: la ruta activa avanza Core, mientras synchronize_rigid_body_mass_properties consulta sim_ptr_, la instancia local sin pasos. set_core_powertrain_telemetry actualiza la masa viva y vuelve a aplicar centro e inercia de esa instancia. Repostar sólo modifica Core.

Posible solución: devolver masa, centro local e inercia desde la misma entidad y el mismo estado que genera la fuerza simplificada. Aplicarlos en una fase coherente del paso de Godot y evitar mezclar valores anteriores y posteriores al consumo. Conservar la ruta Standard y los fallbacks actuales. Revisar el contrato binario si se amplía un frame: emisor y receptor deben coincidir y los consumidores anteriores deben seguir siendo compatibles o quedar versionados explícitamente.

Archivos candidatos a modificar:
- D:/Formula90s/game/crates/formula90-core/src/lib.rs — construcción del frame autoritativo.
- D:/Formula90s/native/src/core/f90_core.cpp — recepción del estado y sincronización con el cuerpo.
- D:/Formula90s/native/src/vehicle/f1_94_rust_vehicle.cpp — sincronización de masa, centro e inercia y setters de combustible.
- D:/Formula90s/native/src/vehicle/f1_94_rust_vehicle.h — declaraciones si cambia la interfaz del vehículo.
- D:/Formula90s/game/crates/vehicle-physics-engine/src/ffi.rs — sólo si la solución necesita ampliar la interfaz de consultas; no utilizar un simulador secundario como autoridad.
- Cabeceras y estructuras del contrato Core–C++ que declaren el frame utilizado: localizar sus propietarios exactos antes de editar y actualizar ambos extremos conjuntamente.

Aceptación: probar 0, 7.6 y 105 kg, consumo continuo, repostaje y reset. Comparar masa, centro e inercia usados realmente por Godot y Rust en el mismo paso. Las consultas de la DLL dieron masas de 605, 612.6 y 710 kg y centros Z de 0.456050, 0.457836 y 0.477339 m. No usar esos valores como golden definitivo si 03 cambia el resolver. El pequeño consumo inicial no demuestra por sí solo el origen de las oscilaciones en ralentí.

## SIMPLIFIED-03 — Centro e inercia respecto al mismo punto

Confirmado: vehicle_mass_properties.rs obtiene centro e inercia del inventario y sustituye la altura del centro por center_of_gravity_height_offset sin trasladar la inercia. La altura del combustible deja de modificar el centro publicado.

Posible solución: para Simplified, utilizar el centro compuesto completo y su tensor referido a ese centro. Si se necesita una altura objetivo, calibrar coherentemente las posiciones de las masas para una carga de referencia; no sustituir sólo una coordenada. Si se cambia el punto de referencia de un tensor, aplicar el traslado correspondiente y no etiquetarlo como inercia respecto a otro centro. No retocar masas o altura del perfil compartido para esconder la inconsistencia. Mantener la aproximación diagonal de Godot claramente identificada y comprobar sus límites; no afirmar que la diagonal son los autovalores del tensor completo.

Archivos candidatos a modificar:
- D:/Formula90s/game/crates/vehicle-physics-engine/src/vehicle_mass_properties.rs — resolver específico o selección explícita de backend.
- D:/Formula90s/game/crates/vehicle-physics-engine/src/suspension_mass_properties.rs — composición y traslado de tensores si requieren adaptación.
- D:/Formula90s/game/crates/vehicle-physics-engine/src/simulation.rs — todas las consultas de centro e inercia del solve simplificado.
- D:/Formula90s/game/crates/vehicle-physics-engine/src/vehicle_configuration/chassis/configuration.rs — revisar fórmulas alternativas de centro y reparto; modificarlas sólo si quedan aisladas por backend.
- D:/Formula90s/game/crates/formula90-core/src/lib.rs — publicación coherente con 02.

Aceptación: suma de masas sin duplicar ruedas ni combustible, centro ponderado, tensor simétrico y físicamente válido, reparto estático compatible con las reacciones por eje. Comprobar referencias de tensor mediante traslación conocida. La auditoría calculó alturas compuestas -0.241493, -0.239737 y -0.220568 m frente a -0.243650 m publicado; diferencias de 2.16, 3.91 y 23.08 mm. Standard debe conservar sus resultados previos.

## SIMPLIFIED-04 — Contrato de contactos rígidos

Confirmado: get_contact_local_position devuelve una posición mundial según la API de Godot. El puente la rota como si fuese local y la envía en local_position. Rust compara después su altura con un umbral local. La palanca tampoco descuenta el centro de masa.

Posible solución: convertir el punto mundial al marco del cuerpo para clasificación; conservar el punto mundial para cinemática. Usar la velocidad del contacto ofrecida por Godot o calcularla desde el centro con una palanca mundial correcta, restando la velocidad del otro cuerpo. Verificar aparte la normal en el backend de Godot utilizado antes de cambiar su transformación. No dar por confirmado un error de normal a partir del nombre del método. Aislar esta interpretación para Simplified sin cambiar el contrato consumido por Standard.

Archivos candidatos a modificar:
- D:/Formula90s/native/src/vehicle/f1_94_rust_vehicle.cpp — collect_underfloor_sample y serialización de contactos.
- D:/Formula90s/native/src/core/f90_core.cpp — selección y transmisión del contrato simplificado si corresponde.
- D:/Formula90s/game/crates/formula90-core/src/underfloor.rs — clasificación y consumo de contactos.
- Estructuras del contrato binario de UnderfloorSample y contactos rígidos: identificar declaración Rust y C++ antes de ampliar o versionar campos.

Aceptación: trasladar toda la escena en horizontal y vertical no debe cambiar clasificación, fuerza ni velocidad relativa del contacto simplificado. Comprobar también rotación del coche y contacto con una plataforma móvil. Standard conserva su contrato y resultado previo.

Referencia: https://docs.godotengine.org/en/stable/classes/class_physicsdirectbodystate3d.html#class-physicsdirectbodystate3d-method-get-contact-local-position

## SIMPLIFIED-05 — Momentos y velocidad del fondo desde el centro

Confirmado: underfloor.rs calcula contacto menos origen y usa esa palanca para velocidad y momento. El resultado se añade a un momento referido al centro y se aplica como torque al cuerpo de Godot.

Posible solución: en Simplified, proporcionar el centro mundial del estado autoritativo. Calcular r = contacto_mundial - centro_mundial; velocidad_del_punto = velocidad_del_centro + velocidad_angular × r; momento = r × fuerza. Garantizar que la fuerza no se aplica además en otro punto duplicando su momento. No usar el selector de aerodinámica como sustituto implícito del selector físico solicitado.

Archivos candidatos a modificar:
- D:/Formula90s/game/crates/formula90-core/src/underfloor.rs — cinemática y acumulación de momentos.
- D:/Formula90s/game/crates/formula90-core/src/lib.rs — entrega del centro y composición de la fuerza y el momento finales.
- D:/Formula90s/native/src/core/f90_core.cpp — revisar aplicación central y torque; modificar sólo si lo exige el contrato.

Aceptación: fuerza conocida en un punto conocido, comparación analítica de r × F y velocidad angular, invariancia al cambiar el origen de representación. Comprobar amortiguación disipativa. El ejemplo de 1000 N verticales y centro inicial Z=0.457836 m implica unos 458 Nm de error de cabeceo; es una estimación matemática, no una medición de la tanda. El defecto sólo actúa cuando el fondo genera fuerza.

## SIMPLIFIED-06 — Geometría física del fondo del Williams

Confirmado: sondas fijas Y=-0.205/-0.165 m frente al centro publicado Y=-0.24365 m. Quedan 38.65/78.65 mm por encima. El contacto se activa a 12 mm: sobre plano horizontal y con coche vertical, exigiría el centro a -26.65/-66.65 mm respecto al suelo. Las bases de las cajas actuales están en -0.18/-0.22/-0.18 m, también por encima del centro publicado. Esto no determina por sí solo dónde debe estar la superficie real.

Posible solución: medir la base física real del modelo en el marco usado por el cuerpo; definir sondas y colisión coherentes en una variante exclusiva de Simplified. Mantener los mismos datos de calibración entre emisión y consumo; eliminar dependencias ocultas de posiciones fijas en ambos lenguajes. Revisar el número de sondas activas y las fórmulas de rake y roll sin confundir telemetría derivada con orientación real. No inventar alturas históricas ni bajar sondas antes de cerrar 04 y 05.

Archivos candidatos a modificar:
- D:/Formula90s/native/src/vehicle/f1_94_rust_vehicle.cpp — construcción y disposición de sondas.
- D:/Formula90s/game/crates/formula90-core/src/underfloor.rs — posiciones asumidas, distancias y offsets de telemetría.
- D:/Formula90s/game/scenes/vehicles/f1_1997_williams/f1_1997_williams_hockenheim_villenueve.tscn — referencia para medir colisionadores; no modificarla globalmente si la consume Standard. Crear variante o selección exclusiva simplificada si resulta necesaria.
- D:/Formula90s/game/data/vehicles/f1_1997/williams/profiles/hockenheim/underfloor_contact.json — referencia de umbrales; editar sólo mediante datos exclusivos de Simplified si se requiere recalibrar.
- D:/Formula90s/game/crates/vehicle-physics-engine/src/vehicle_configuration/underfloor_contact/mod.rs y D:/Formula90s/game/crates/vehicle-physics-engine/src/underfloor_contact_configuration.rs — sólo si hace falta representar la calibración específica.

Aceptación: medir holgura real en Godot mediante el servidor obligatorio y lanzador canónico. Compresión progresiva sobre plano y bache: el fondo debe reaccionar a la separación prevista antes de penetración incompatible. No duplicar soporte ni impulso entre colisión rígida y contacto elástico. Comprobar límites de fuerza, liberación sin rebote creciente y conservación del comportamiento Standard.

## Validación transversal y cierre

Antes de implementar, capturar una referencia reproducible del backend Standard en este HEAD y de la ruta simplificada actual. No actualizar goldens Standard para aceptar cambios de comportamiento causados por este trabajo. Comparar fuerzas, momentos, masas, centro, inercia y trayectorias, con entradas y condiciones iguales. Revisar también combinaciones de selectores de suspensión y aerodinámica: cambiar uno no debe activar accidentalmente el alcance del otro.

Después de los seis ítems, repetir volanteo en recta, reposo con motor al ralentí y la secuencia de baches de Fuji. Registrar orientación real del cuerpo, velocidades angulares, cargas por rueda, carrera y velocidad de suspensión, contactos, energía e impulsos. En la disposición de dos sondas longitudinales, el roll del fondo se fuerza a cero y no demuestra carrocería horizontal. Separar causalidad comprobada de mejoría aparente.

El acoplamiento por tick entre Godot y Rust sigue siendo una hipótesis de contribución, no un séptimo fallo confirmado: los subpasos de suspensión reutilizan contactos y cinemática del comienzo del tick. Comparar 120/240/480 Hz como diagnóstico; no autoriza rediseñar el integrador ni extender el alcance a Standard. Abrir un nuevo ítem de corrección sólo si aparecen evidencias reproducibles.

Para el futuro rebuild, verificar HEAD, limpiar Cargo, SCons y game/.godot conforme al protocolo y no reutilizar productos entre ramas. BUILD_SOURCE y los binarios deben corresponder al código actual. Usar scripts/run_f1_94.ps1 como lanzador canónico y conservar sus controles de paridad; respetar la sesión Williams prevista. No lanzar directamente mediante MCP para saltar esos controles. Si no hay puente vivo, declarar la limitación y no presentar pruebas estáticas como validación de conducción.

Definición de terminado: seis hallazgos corregidos exclusivamente en Simplified, pruebas de cada contrato superadas, Standard sin regresiones frente a su referencia, binarios y BUILD verificados cuando se implemente, documentación actualizada, revisión y aprobación humana de conducción. Actualmente todos los ítems siguen pendientes; sólo se ha reemplazado este documento.
