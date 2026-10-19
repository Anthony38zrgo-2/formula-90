# Instrucciones — siguientes pasos de configuración

Fecha: 2026-10-18. Rama: `main-clean`.
HEAD de referencia: `edc75cd09668012c055caab2880c74bee8ce0f1e`.
`game/BUILD_SOURCE`: `edc75cd09668012c055caab2880c74bee8ce0f1e`.

Estado: la corrección de contacto y acoplamiento (`99d9c847`) y la calibración dependiente del recorrido (`edc75cd0`) están commiteadas. El rebuild limpio, la validación canónica y una vuelta autónoma en Fuji pasaron. Falta la aceptación humana de conducción y las decisiones de setup que se describen abajo.

## Evidencia de referencia

- `scratch/simplified_suspension_delivery_20261018/delivery_report.md`
- `scratch/simplified_suspension_delivery_20261018/inertia_and_ride_height_audit.md`
- `scratch/simplified_suspension_delivery_20261018/williams_autonomous_20261018_221227/fuji_report.json`
- `scratch/simplified_suspension_delivery_20261018/williams_autonomous_20261018_221227/analysis_summary.json`
- `scratch/simplified_suspension_delivery_20261018/test_logs/`

Resultado autónomo aceptado: 1 vuelta en 82,292 s, 304,71 km/h, inclinación máxima 5,333°, sin descarga de las cuatro ruedas y sin vuelco. `get_normal_forces` ya publica cargas reales por rueda.

## 1. Aceptación humana de conducción

Ejecutar:

```powershell
scripts\run_f1_97.ps1
```

Probar, en este orden:

1. Recta a velocidad creciente y frenada progresiva.
2. Curva de radio constante a baja, media y alta velocidad.
3. Cambios rápidos de dirección y correcciones de volante.
4. Pianos y bordillos, con pérdida y recuperación de apoyo.
5. Aceleración a fondo desde baja velocidad y frenada fuerte.
6. Cambios de barra delantera/trasera, si se van a usar como ajuste.

Registrar la sensación de transferencia de peso por eje, el asentamiento del balanceo y el cabeceo. No usar ayudas de estabilidad para tapar el síntoma. Si algo se siente inconsistente, anotar velocidad, curva, mando y tren afectado antes de tocar valores.

## 2. Altura de carrocería y centro de masa

La auditoría muestra que el +50 mm deja el CG en ~0,327 m y la holgura del fondo en 0,0676 m delante / 0,0620 m detrás, frente a 0,030 / 0,045 m nominales. El margen estático de vuelco baja a ~2,44–2,55 g.

Siguiente paso recomendado: evaluar una reducción de **25–30 mm** de altura estática.

- Ajustar las longitudes libres de muelle para conservar la precarga y el asiento:
  - delante: reducir ~26,3 mm para −30 mm de altura;
  - detrás: reducir ~11,1 mm para −30 mm.
- Recalcular con la tabla de relación de movimiento, no con la constante de diseño.
- Actualizar `suspension.json`, `profile_provenance.json` y el digest del manifiesto.
- Verificar la holgura estática con la medición headless y repetir la vuelta autónoma.
- No bajar la altura por debajo de la holgura nominal Q38 (0,030 / 0,045 m) sin evidencia de la pista real.

## 3. Eje trasero: ley decreciente y topes

La relación de movimiento trasera cae de ~0,50 en rebote a ~0,31 en compresión; la rigidez equivalente cae con ella. Los amortiguadores se subieron a 14.000/22.000 Ns/m (ζ ≈ 0,64/1,01 en el asiento), pero la rueda trasera izquierda todavía alcanzó 148,96 mm en Fuji, dentro del sobrepaso del tope duro.

Siguientes comprobaciones:

1. Auditar la geometría del rocker trasero: brazo de amortiguador y de pushrod, y si la ley decreciente es intencional o un artefacto de reconstrucción.
2. Si se mantiene, recalibrar el inicio y la progresividad del tope trasero para que no sea el primer elemento que sostiene la transferencia.
3. Validar con una rampa de transferencia de carga y con el replay de entradas grabadas, no solo con inclinación máxima.

## 4. Configuración de barras

Simplified usa `suspension.front/rear.arb_ratio` (1,00/0,80); Standard usa `geometry_physical.front/rear.arb_ratio` (0,15/0,09). Es una divergencia intencional y documentada, pero el mismo perfil representa dos coches distintos.

Decidir una de estas opciones antes de la siguiente temporada de ajuste:

- Mantener la divergencia y publicar la rigidez efectiva por eje en telemetría.
- Igualar Simplified y Standard en un único conjunto de barras y documentar el cambio de equilibrio.
- Añadir una sección explícita de barras Simplified en el perfil, separada de la geometría física.

No cambiar los valores sin repetir la vuelta autónoma y la prueba humana.

## 5. Inercia y sensación de peso

No cambiar el tensor actual (396,03 / 462,18 / 104,90 kg·m²) sin un objetivo validado. El modo de balanceo (~9 Hz) está dominado por la rigidez delantera y la barra delantera, no por la inercia.

Si después de los puntos 2–4 el coche sigue sintiéndose ligero:

1. Hacer un barrido de sensibilidad de inercia (×1,0 / ×1,15 / ×1,3) y medir frecuencia y asentamiento de balanceo, cabeceo y guiñada.
2. Antes de subir la inercia, evaluar bajar la rigidez delantera o la barra delantera, que es la palanca dominante del modo de balanceo.
3. Cualquier cambio de inercia debe ir con procedencia y una prueba de manejo, no solo con una estimación.

## 6. Telemetría y validación

La carga por rueda ya está disponible en `get_normal_forces` y en `physical_diagnostics.jsonl`. Usarla para comparar:

- carga normal por rueda y suma total;
- compresión y velocidad de suspensión;
- fuerzas de muelle, amortiguador, barra y topes;
- pose, velocidades lineal/angular y centro de masa;
- contactos rígidos y de fondo.

Mantener 120 Hz de física y 60 Hz de control. Registrar siempre el `BUILD_SOURCE` y el HEAD del run.

## 7. Inestabilidad de importación de Godot

El primer import tras un rebuild limpio puede crashear con `0xC0000005`. El Visor de eventos lo atribuye a `Godot_v4.7.1-stable_win64.exe`, no a las DLL de formula90s. Si ocurre:

1. Cerrar Godot.
2. Borrar `game/.godot`.
3. Repetir `scripts\run_f1_97.ps1 -ValidateRuntimeOnly`.
4. Confirmar `[formula90s] GDExtension bootstrap ready` y la paridad BUILD/HEAD.

No interpretar ese crash como un fallo de las físicas sin evidencia del módulo faulting.

## 8. Checklist de rebuild y entrega

```powershell
git rev-parse HEAD
Get-Content game\BUILD_SOURCE
scripts\build_windows.ps1 -Configuration debug
scripts\run_f1_97.ps1 -ValidateRuntimeOnly
scratch\simplified_suspension_delivery_20261018\run_williams_autonomous_validation.ps1
git status --porcelain
```

- No commitear `game/BUILD_SOURCE`, DLLs, `game/.godot` ni artefactos de build.
- Mantener commits atómicos y staging por rutas explícitas.
- Preservar los cambios ajenos de audio (`grand_prix_sampler.rs`), el test de `recorded_engine_profile.rs` y los assets no rastreados.
- No cambiar de rama, no resetear y no reutilizar DLLs de otra rama.
- La aprobación humana de conducción es obligatoria antes de declarar cerrado el ajuste de peso y transferencia.

## Límites

Standard debe conservar sus ecuaciones, referencias, contactos y masa. El mundo físico acoplado independiente queda fuera de este ajuste. No seleccionar correcciones por nombre de coche ni aplicar cambios globales al perfil compartido para corregir solo Simplified.
