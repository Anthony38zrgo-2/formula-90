# Roadmap

La Fase 2 integra el V10 procesado, herramientas C++ de sprites y validación, el banco original `v10_prototype` y DSP C++ en runtime.

- Fase 1: bootstrap jugable y probado.
- Fase 2: herramientas validadas de sprites y audio, sin entrar en runtime.
- Posterior: circuito, rivales, vueltas y contenido original.

## Backlog — Editor de pistas y pipelines independientes

Refactorizar la autoría de circuitos para reducir el coste de iteración manual.
No iniciar esta iniciativa sin planificación y aprobación explícita.

Objetivo arquitectónico:

- Separar el pipeline de producción de assets reutilizables del pipeline de
  autoría y compilación de pistas.
- Mantener un Asset Registry único: las pistas referencian assets por ID
  semántico, nunca por rutas de archivo.
- Convertir `track.source.svg` en la única fuente editable de verdad; el JSON
  normalizado será generado, nunca editado manualmente.
- Crear un editor local, ligero y basado en navegador (Vue 3 + SVG.js), con
  modos Objects y Terrain sobre el mismo documento.
- Permitir colocar, mover, rotar, escalar, duplicar y borrar instancias; las
  instancias ancladas al terreno conservan X/Z y derivan Y del heightfield.
- Separar Save de Build: guardar sólo persiste SVG; Validate & Build será una
  acción humana explícita, validada por hash y nunca invocará Blender con SVG
  inválido o sin aprobación humana.
- Reducir Blender a compilador headless determinista: consume el JSON
  normalizado, genera pista/terreno/colisión, resuelve assets registrados y
  exporta GLB. Godot permanece como consumidor runtime.

MVP y validación:

1. Auditoría y clasificación KEEP/MOVE/REFACTOR/DEPRECATE de los pipelines
   actuales.
2. Asset Registry con validación de IDs duplicados, rutas inexistentes y
   previews.
3. Esquema SVG, parser, normalizador, validación y bloqueo por estado DIRTY.
4. Servidor local y editor 2D para assets, objetos, terreno, undo/redo y guardado.
5. Build explícito con Blender headless, prueba de ground-lock y carga de la
   pista resultante en Godot.

Fuera del MVP: Three.js, editor 3D, scatter automático, road painter,
banking/camber, pitlane, IA, climatología y un sustituto de Blender.

## Backlog — Migración de Físicas GEVP a Rust

Documento de referencia (eliminado): la migración GEVP→Rust está completa; la fuente de verdad para F1-94 es ahora `game/data/vehicles/f1_94/f1_94_physics.json` (ver `PROJECT_STATE.md`).

Migración del núcleo físico del vehículo (GEVP GDScript `vehicle.gd` / `wheel.gd`) a un crate puro en Rust (`game/physics/engine`), siguiendo el patrón arquitectónico desacoplado de `vehicle_audio_engine` y `skybox_engine`.

Objetivos clave:
- **Modelo Tri-Raycast (3 Raycasts por rueda)**: Elimina la ceguera a pianos/bordillos y proporciona transiciones de carga suaves y contacto multicapa.
- **Telemetría Paritaria**: Generación directa en Rust del esquema CSV de 25 columnas de `TelemetryManager` para validación cruzada.
- **Simulación Determinista Offline**: Batería de tests `cargo test` para aceleración (0-100, 0-200 km/h), frenada (200-0 km/h con ABS), skidpad y absorción de bordillos sin abrir Godot.

