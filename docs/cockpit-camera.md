# Cámara de cockpit del F1 2030 V10 base

## Uso

La cámara se limita al F1 2030 V10 original. La tecla física `C` recorre chase, T-cam y cockpit. En cockpit se oculta `DriverHeadAndNeck` y permanecen visibles brazos, guantes, torso y piernas; la cabeza se restaura al salir.

`F9` abre y cierra los ajustes de movimiento mientras cockpit está activo. Los cambios se aplican inmediatamente a la configuración compartida entre piloto y cámara. **Guardar preferencias** conserva únicamente los seis ajustes en `user://cockpit_camera_preferences.json`; **Restablecer** recupera los valores del archivo de configuración cargado al iniciar. Es necesario guardar después de restablecer si se desea conservarlos para la próxima sesión.

| Ajuste | Efecto |
| --- | --- |
| Estabilidad de la mirada | Compensación del movimiento del chasis respecto a la referencia de carretera. |
| Aceleración y frenada | Intensidad del impulso longitudinal y su recuperación. |
| Feedback de baches verticales | Respuesta vertical y cabeceo por diferencias entre ejes. |
| Balanceo por baches | Inclinación y respuesta lateral ante diferencias entre ruedas. |
| Absorción del movimiento | Corrección limitada de la posición del ojo durante baches. |
| Mirar hacia la trayectoria | Orientación parcial hacia la velocidad real; inicialmente desactivada. |

## Punto de vista y referencia visual

`DriverEyePoint` está unido al hueso de la cabeza, en el centro de la abertura del visor y 35 mm detrás de su superficie frontal. El modelo no contiene globos oculares; el visor es la referencia geométrica. La elevación artificial de 14 cm conserva el encuadre aprobado para ver la trazada. Esa elevación sigue la referencia de la carretera, sin describir un arco adicional cuando el piloto inclina la cabeza por una fuerza longitudinal.

La mirada compensa el cabeceo y balanceo del chasis respecto a una referencia filtrada de la superficie. Sigue gradualmente pendientes y peraltes en vez de mantener siempre un horizonte mundial horizontal. Los tiempos de seguimiento son 0.22 s para pendiente y 0.35 s para peralte. Las compensaciones independientes son 0.85 y 0.90, multiplicadas por Estabilidad de la mirada. Se mantiene un límite de 45° para la corrección del cuello ante posiciones extremas.

## Señales que producen movimiento

### Aceleración y frenada

`long_g` genera un impulso inicial filtrado durante 0.07 s y limitado suavemente a una excitación de 1.8 G. La compensación se adapta durante 0.35 s bajo fuerza sostenida y se libera en 0.08 s al reducir o invertir la fuerza. No genera un impulso contrario por soltar el pedal. La respuesta conserva 1.25° por G antes del límite suave, frecuencia de 2.7 Hz y amortiguación de 1.0. Una entrada sintética de 3 G alcanza ahora cerca de 1.21° de cabeceo y se recupera a menos de 0.05° tras 1.5 s.

Cuando cambia la marcha indicada por la telemetría, el impulso longitudinal se reduce inicialmente un 75% y recupera su intensidad de forma gradual durante 0.22 s. Así se amortigua el golpe breve de una transición de marcha sin modificar la física del vehículo ni las respuestas verticales y laterales. La primera lectura de la marcha establece la referencia y no activa esa reducción.

### Baches verticales, cabeceo y balanceo

Las fuerzas G lateral y vertical de telemetría no alimentan estos canales. Dos filtros separan el recorrido lento de las suspensiones de sus variaciones rápidas; los residuos menores de 1.5 mm por rueda se descartan. La señal se convierte a excitación equivalente sin cambiar la física del vehículo.

Los contactos centrales de las cuatro ruedas aportan normales y posiciones de la superficie. Una diferencia de altura respecto a la superficie local o un salto del centro de contacto confirma el bache, con entrada y salida filtradas durante 0.05 s. Sobre una superficie plana, una transferencia rápida de carga por sí sola no activa el efecto. La confirmación aumenta entre 1.5 y 6 mm. Con contactos incompletos se atenúa; sin una interfaz de contactos se usa la señal de suspensión y la orientación del vehículo como referencia.

La respuesta se divide en tres componentes:

- El movimiento común de las ruedas genera una pequeña traslación vertical amortiguada del punto de vista, sin añadir un cabeceo artificial por un bache simétrico.
- La diferencia entre ejes genera cabeceo del cuello y cabeza, con ganancia de 0.2° por unidad equivalente y límite suave de 0.6°.
- La diferencia izquierda/derecha genera balanceo, con ganancia de 0.9° por unidad equivalente y límite suave de 3°.

Cuando hay velocidades lineal y angular válidas, la aceleración calculada en el asiento incluye la variación de velocidad, la aceleración angular y el componente centrípeto del punto. Se elimina su componente sostenida y se mezcla un 35% de su señal vertical transitoria, únicamente cuando la suspensión y los contactos confirman el bache. No se añade balanceo por la fuerza centrífuga de una curva.

El canal lateral conserva filtro de 40 ms, respuesta de 2.3 Hz y amortiguación de 1.0. El vertical usa 60 ms, 1.9 Hz y amortiguación de 1.1. La traslación vertical añadida usa 2 mm por unidad equivalente. Las ganancias individuales permiten reducir o desactivar cada tipo de feedback.

### Corrección posicional

La cámara sigue el ojo animado y añade una corrección transitoria, activada mediante envolventes suaves de los canales de baches. Filtra el movimiento lateral en 40 ms y el vertical en 60 ms. No retrasa el avance del auto: elimina el componente de movimiento a lo largo de la tangente de carretera.

La corrección final combinada tiene límites suaves de 2 cm verticales y 1 cm laterales, multiplicados por Absorción del movimiento, inicialmente 0.8. Cuando cesa el bache vuelve al ojo elevado. Entrar a cockpit reinicia la corrección posicional; un teletransporte o una interrupción de actualización reinicia la inercia y la historia de interpolación.

## Actualización compartida

`cockpit_driver_motion_state.gd` calcula el estado una vez por paso físico. El proyecto funciona a 120 pasos físicos por segundo. El modificador de cuello y cabeza y el rig de cámara leen e interpolan las mismas respuestas de fuerzas, sin integrarlas durante el renderizado. El anclaje de cámara, la orientación del vehículo y el ojo usan la transformación actual que muestra el cockpit. El rig permanece independiente de la transformación heredada del vehículo y copia ese anclaje después de actualizar el esqueleto.

La posición global del vehículo no se vuelve a interpolar exclusivamente para la cámara. Hacerlo mientras el habitáculo utiliza la posición actual produce una separación variable con la velocidad y la fase de renderizado, percibida como desplazamientos hacia delante y atrás. La regresión de sincronización adelanta el vehículo entre el muestreo físico y la actualización del esqueleto a 0, 30 y 80 m/s, y exige que el anclaje de cámara conserve su posición respecto al ojo visible.

Las rotaciones se distribuyen entre cuello y cabeza con fracción de cuello de 0.65, conservando las longitudes de los huesos. El piloto mantiene esta animación en las vistas externas. Los límites de fuerza y corrección usan saturación suave para evitar cambios bruscos al alcanzar un tope.

## Orientación opcional hacia la trayectoria

Se calcula el ángulo entre la dirección del vehículo y su velocidad, no el ángulo del volante. La intensidad inicial es cero. Al activarla, su influencia crece entre 3 y 10 m/s de avance, se filtra en 0.15 s y tiene límite suave de 8°. Al detenerse o retroceder retorna al centro.

Esta opción sigue el concepto documentado de [DriverRotateHead de iRacing](https://support.iracing.com/support/solutions/articles/31000133489-undocumented-hidden-feature-active-yaw-axis-cockpit-view-driverrotatehead-): orientar parcialmente la vista hacia el vector de velocidad durante el deslizamiento. La implementación es propia y no reproduce un algoritmo interno de iRacing.

## Configuración y validación

Los valores base están en `game/data/cameras/formula_one_2030_cockpit_camera.json`. `cockpit_camera_configuration.gd` valida valores finitos, rangos y orden de umbrales. La interfaz solo modifica sus seis preferencias; el resto del ajuste permanece en el archivo de datos. `set_cockpit_force_response_strength()` conserva el control global de intensidad del piloto.

La suite `game/tests/test_cockpit_camera.gd` comprueba ubicación del ojo, encuadre elevado, visibilidad, horizonte, longitud de huesos, signos e impulso longitudinal, recuperación, inversión de fuerza, filtrado de baches, límites y teletransporte. También comprueba el límite de 1.4° para una entrada sintética de 3 G, la reducción de impulsos al subir o bajar de marcha y la ausencia de un segundo cabeceo durante aceleración sostenida. Comprueba seguimiento de pendiente/peralte, ausencia de rebote por fuerzas laterales/verticales sostenidas, contactos reales sobre plano y obstáculo, orientación opcional, controles F9 y preferencias compartidas. Reproduce la misma secuencia física con muestreo de render a 30, 60, 120 y 144 Hz y comprueba que el estado final sea idéntico.

En la revisión de 2026-10-01 pasaron la suite de cockpit, la regresión de manos y el smoke canónico `scripts/run_f1_94.ps1 -Smoke`, incluyendo el ciclo real con `C`. Las pruebas sintéticas y las capturas controladas no sustituyen la evaluación de comodidad al conducir sobre pista, frenar y pasar pianos.

Proveniencia de la revisión: rama `main-clean`, HEAD `56b2a1d493053bb38232bbe73928e02e913d59c8`. Los cambios previos de manos, suavizado de cámara, BUILD_SOURCE, DLLs e importaciones se inventariaron antes de esta revisión. La ejecución canónica verificó BUILD/HEAD de los binarios existentes; esta revisión modifica GDScript y configuración.

La captura original a 1920×1080 midió 0.54981 m entre el ojo y la pista. La elevación de 0.14 m equivale al 25.46% de esa altura. Las capturas originales de la vista elevada permanecen en `scratch/cockpit_driver_camera/captures_raised/`.
