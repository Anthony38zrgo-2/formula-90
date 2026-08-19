# Sprint retrospective: integración visual Jordan 197

- Fecha: 2026-08-13
- Alcance: diagnóstico, corrección visual, smoke tests y prevención operativa
- Resultado: PASS
- Incidente relacionado: `J197-VIS-001`

## Objetivo del sprint

Integrar el nuevo Jordan 197 sin cambiar su física y conseguir que el chasis,
los ejes visuales, las ruedas y la dirección física describieran el mismo coche.
La definición de terminado exigía:

1. identificar la causa antes de editar;
2. reproducirla mediante una comprobación determinista;
3. aplicar un único cambio visual reversible;
4. proteger el contrato con un smoke test semántico;
5. validar Jordan 197, Jordan 191, HUD, importación y captura visual;
6. preservar los GLB y el trabajo local no relacionado.

## Resultado entregado

El GLB canónico conserva su convención de asset `frente=+Z`, `arriba=+Y` e
`izquierda=+X`. La escena del vehículo es la única propietaria de la conversión
al runtime GEVP, cuyo frente es `-Z`. `ChassisVisual` aplica un yaw de 180 grados
con determinante positivo; no es una escala negativa.

El delta medido fue:

```text
baseline:  visual_forward dot physical_forward = -1
candidato: visual_forward dot physical_forward = +1
```

No se modificaron RayCast, colisiones, suspensión, masas, neumáticos, dirección,
motor ni handling.

## Causa raíz

La integración mezcló dos espacios de coordenadas sin declarar quién era dueño
de la conversión:

```text
asset GLB         frontal +Z
scene/runtime     frontal -Z
```

El generador y el prompt afirmaban que el importador de Godot convertiría
semánticamente `+Z` a `-Z`. La escena, por tanto, montó `ChassisVisual` con basis
identidad. Godot preservó la orientación del asset. Al mismo tiempo, los RayCast
delanteros estaban correctamente situados en `Z=-1.433` y los traseros en
`Z=+1.640`.

El resultado era un chasis visual girado 180 grados respecto al cuerpo físico:
la cámara trasera veía el morro y las ruedas delanteras/traseras parecían
intercambiadas respecto a la carrocería.

### Cinco porqués

1. ¿Por qué la cámara veía el morro? Porque el chasis visual apuntaba a `+Z`.
2. ¿Por qué el chasis apuntaba a `+Z`? Porque se instanció con transform identidad.
3. ¿Por qué no se aplicó yaw 180? Porque se asumió que Godot convertiría el frente
   semántico del GLB.
4. ¿Por qué sobrevivió esa suposición? Porque el manifiesto guardaba datums de la
   fuente, pero no declaraba `asset_forward`, `runtime_forward` ni el dueño de la
   conversión.
5. ¿Por qué llegó a runtime? Porque el smoke comprobaba existencia, scripts,
   materiales y superficies, pero no dirección, orden de ejes ni correspondencia
   semántica de las ruedas.

## Evidencia decisiva

- La captura mostraba el defecto a `0 km/h` y en neutral.
- La fuente validada tenía `nose_z=+2.426`, `front_axle_z=+1.4329` y
  `rear_axle_z=-1.6401`.
- El runtime definía el eje delantero en `Z=-1.433` y el trasero en `Z=+1.640`.
- La cámara calculaba el frente físico como `-basis.z`.
- `ChassisVisual` usaba basis identidad antes del fix.
- La documentación arquitectónica previa ya establecía yaw 180 para convertir
  assets `+Z` al runtime `-Z`.

## Hipótesis rechazadas (Context GC)

```text
REJECTED HYPOTHESIS:
La suspensión o las fuerzas dinámicas deforman el coche.
EVIDENCE:
El defecto ya existe parado, a 0 km/h y en neutral.
NEW VERIFIED FACT:
La firma es estructural/visual y no depende del estado dinámico.
STATUS: Do not revisit without new evidence.
```

```text
REJECTED HYPOTHESIS:
Godot convierte automáticamente el frente semántico +Z del GLB a -Z.
EVIDENCE:
Datums de fuente + transform identidad producen dot=-1; yaw 180 produce dot=+1
y la captura candidata muestra la zaga desde la cámara trasera.
NEW VERIFIED FACT:
El importador preserva la orientación semántica; la escena visual posee la
conversión en este pipeline.
STATUS: Do not revisit without new evidence.
```

```text
REJECTED HYPOTHESIS:
El crash 0xC0000005 prueba que el fix o el smoke están rotos.
EVIDENCE:
Godot falló antes de las aserciones con `Failed to open user://logs`; la ACL daba
al usuario sandbox sólo lectura. Fuera del sandbox el mismo smoke terminó PASS.
NEW VERIFIED FACT:
Un crash previo a las aserciones es fallo del validador/entorno e INCONCLUSIVE
para el candidato.
STATUS: Do not revisit without new evidence.
```

```text
REJECTED HYPOTHESIS:
Un `Get-FileHash` sin valor demuestra que el manifest tiene un hash distinto.
EVIDENCE:
El cmdlet devolvió acceso denegado y `$null`; ejecutado con acceso de lectura,
los tres hashes coincidieron.
NEW VERIFIED FACT:
Primero se valida que la lectura tuvo éxito; sólo después se comparan hashes.
STATUS: Do not revisit without new evidence.
```

## Qué salió bien

- La captura inicial permitió clasificar rápido el defecto como visual.
- Se compararon datums del asset, escena, cámara y contrato arquitectónico antes
  de cambiar código.
- Se mantuvo el cambio causal mínimo: un solo basis en `ChassisVisual`.
- `scene-safety` preservó recursos, `node_paths`, owners, RayCast y colisiones.
- El smoke se convirtió de una prueba de presencia a una prueba semántica.
- La validación aumentó de alcance sólo después del PASS focalizado.
- Se preservaron los GLB modificados y el resto del worktree del usuario.
- La captura final confirmó visualmente lo que ya probaba el dot product.

## Qué salió mal y cómo se resolvió

| ID | Fallo | Impacto | Resolución | Prevención permanente |
|---|---|---|---|---|
| W1 | Contrato de ejes implícito y contradictorio | El coche se montó al revés | Se declaró asset `+Z`, runtime `-Z` y escena como dueño del yaw 180 | `coordinate_contract` obligatorio y guardas en `AGENTS.md` |
| W2 | `ChassisVisual` usaba identidad | Morro y cola invertidos respecto a física/cámara | Basis `diag(-1,1,-1)` conservando offset Y | Smoke compara frente visual y físico |
| W3 | Smoke superficial | Materiales y nodos podían pasar con el coche al revés | Se añadieron dirección, datums, ejes y GLB de rueda | Toda promoción 3D debe probar semántica, no sólo existencia |
| W4 | Comentario incorrecto en el generador | Reforzó una suposición falsa | Se corrigió la documentación y la salida del manifest | El generador no puede atribuir conversiones no medidas a Godot |
| W5 | Prompt de agentes repetía la misma ambigüedad | Futuras integraciones repetirían el defecto | Se corrigió el prompt de normalización | Fuente y runtime deben documentarse por separado |
| W6 | `agentdb` no estaba disponible | No se pudo consultar el índice de incidentes | Se continuó con evidencia local y se registró la limitación | Si `agentdb` falta, buscar `resolved-incidents` y `COMMON_ERRORS`; no interpretar ausencia de herramienta como “sin antecedentes” |
| W7 | Godot intentó escribir `user://logs` en una ruta sólo lectura | Dos ejecuciones terminaron con signal 11/0xC0000005 antes del test | Se verificó ACL y se ejecutó con un user dir escribible/fuera del sandbox | Preflight de escritura de `user://`; crash previo a assertions = INCONCLUSIVE |
| W8 | Se confundió acceso denegado con hash mismatch | Generó una falsa alarma de integridad | Se comprobó éxito de lectura y luego el hash | Nunca comparar `$null` ni continuar tras error de IO |
| W9 | Un parche amplio dependía de texto mojibake en PowerShell | `apply_patch` no encontró el contexto | Se dividió el parche y se usó un ancla ASCII estable | Parches pequeños; evitar anclas con texto corrupto/locale |
| W10 | Una expresión PowerShell compacta sin espacios/paréntesis dio falso FAIL | La comprobación imprimió dot=1 pero lanzó error | Se reescribió con paréntesis y operadores separados | Validadores legibles; resultado numérico y condición deben concordar |
| W11 | `--help | Select-Object -First` cerró el pipe temprano | El comando se clasificó erróneamente con exit 1 | Se ejecutó `--help` sin truncar el pipe | No usar consumidores que cierren stdout para decidir salud del proceso |
| W12 | Movie Maker con destino PNG creó 8 frames y WAV | Añadió artefactos temporales al worktree | Se conservó una captura y se eliminaron sólo los temporales creados | Definir ruta temporal/limpieza explícita para capturas |
| W13 | Worktree ya contenía assets modificados y archivos sin seguimiento | Riesgo de sobrescribir trabajo ajeno | Se inspeccionó status/diff y no se regeneraron GLB | Todo preflight debe identificar ownership y limitar los archivos editados |

## Cambio de pruebas

El smoke del Jordan 197 ahora falla si:

- falta el manifest o sus datums de eje no son válidos;
- `visual_forward dot physical_forward < 0.99`;
- el eje delantero o trasero transformado difiere más de 1 cm del RayCast;
- una rueda delantera no instancia el GLB delantero;
- una rueda trasera no instancia el GLB trasero;
- se pierden scripts, jerarquía, materiales o superficies.

El smoke quedó incorporado a `scripts/test_windows.ps1` para que no dependa de
una ejecución manual.

## Validación final

| Validación | Resultado |
|---|---|
| Cálculo estático baseline | `dot=-1`, FAIL esperado |
| Cálculo estático candidato | `dot=+1`, PASS |
| Smoke Jordan 197 | PASS |
| Smoke Jordan 191 | PASS |
| Bootstrap/HUD compositor | PASS |
| Carga del editor/importación | PASS |
| Apertura headless de la escena | PASS |
| Hashes de chassis/front/rear | PASS |
| Runtime import cache freshness | PASS |
| Captura desde cámara trasera | PASS |

## Reglas resultantes para futuras implementaciones

1. Nunca inferir una conversión semántica de ejes por el formato o el importador.
2. Derivar el frente del asset de `nose-tail` o `front_axle-rear_axle`.
3. Derivar el frente físico de los centros de los RayCast, no de nombres.
4. Declarar exactamente un dueño de la conversión asset→runtime.
5. Probar `dot`, ejes y piezas semánticas antes de una revisión visual.
6. Si el defecto existe parado, falsificar ensamblaje visual antes de ajustar física.
7. Un crash de la herramienta no es un FAIL del candidato sin una aserción ejecutada.
8. Verificar que `user://` es escribible antes de lanzar Godot headless.
9. Distinguir `read failed` de `value mismatch` en hashes y manifests.
10. Mantener la corrección visual debajo del dueño visual y fuera de GEVP.

## Artefactos de evidencia

- Escena: `game/scenes/vehicles/jordan_197/jordan_197.tscn`
- Smoke: `game/tests/smoke_test_jordan_197_handling_scene.gd`
- Manifest: `game/assets/models/vehicles/jordan_197/vehicle_manifest.json`
- Generador: `tools/asset_pipeline/generate_vehicle_runtime.py`
- Captura candidata: `reports/jordan_197_orientation_candidate.png`

