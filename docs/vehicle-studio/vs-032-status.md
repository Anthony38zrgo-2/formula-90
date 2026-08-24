# VS-032 — Wheelbase y anchura global

Status: READY_FOR_HUMAN_GATE

## Implementado

- Deformación longitudinal por tramos, anclada en ambos ejes y centrada en el origen.
- Escala lateral interpolada entre front track y rear track.
- Aplicación determinista a mallas y datums dentro de Blender factory-startup.
- Preservación estricta de vértices, polígonos y pertenencia de capas UV.
- Endpoint POST /api/build/materialize y botón Materializar variante en Blender.
- Salida content-addressed en tools/vehicle_studio/staging; nunca publicada.

## Evidencia

- Suite Python: 62 pruebas OK.
- Vue: 6 pruebas OK.
- vue-tsc y build Vite: OK.
- Variante Williams real combinada:
  - wheelbase: 3.06668357625 m (+5 %)
  - front track: 1.6402760197 m (+3 %)
  - rear track: 1.4941207596 m (-2 %)
  - BuildIR: A52438A12DC955ECA0E7605A4128061EC4A4F18CC1B28770B09291961BAD1CE2
  - fuente sin cambios: sí

## Límite visible

El botón crea el Blend deformado y muestra su ruta. El preview web todavía consume el GLB fuente; no se sustituye hasta que el exportador GLB staged esté implementado. Radio/anchura de neumáticos y contacto con suelo pertenecen a VS-033/034.
