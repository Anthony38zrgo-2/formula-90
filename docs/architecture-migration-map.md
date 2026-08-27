# Mapa físico de migración arquitectónica — Formula90s

> **Backlog:** ARCH-002.
> **Estado:** mapa transitorio; no ejecuta movimientos.
> Fecha: 2026-08-26.
> Procedencia: rama `f1-94` @ `d7d8a08fca460341cf1591f508bd58be4c7d1c14`.
> Arquitectura canónica: `docs/decoupling-blueprint.md`.

---

## 1. Propósito

Este documento traduce la arquitectura objetivo a las rutas existentes. Su
objetivo es que cada migration slice sepa qué conserva, qué mueve, qué divide
y qué debe revisar antes de tocar el árbol.

No autoriza `git mv`, borrados ni regeneración de outputs. Todo movimiento se
ejecuta en un backlog item posterior, con staging explícito y sin mezclar
dominios.

---

## 2. Leyenda

| Acción | Significado |
|---|---|
| **KEEP** | La ruta ya pertenece a la zona correcta |
| **MOVE** | La ruta tiene una responsabilidad clara, pero vive en otra zona |
| **SPLIT** | La ruta mezcla fuentes, tooling, outputs o runtime |
| **REVIEW** | No se mueve hasta confirmar consumidores y procedencia |
| **RETIRE** | Candidato a eliminación después de demostrar que no tiene consumidores |

Regla transitoria:

> Una ruta legacy continúa siendo válida hasta que su slice complete fuente,
> herramienta, preview, promoción y consumidor runtime.

No se mantienen dos fuentes canónicas después del cutover de un slice.

---

## 3. Zonas objetivo reales

```text
Runtime y contenido cargable
  game/
  game/assets/
  game/sounds/

Tooling offline
  tools/

Fuentes editables
  source-assets/

Formatos compartidos reales
  formats/

Experimentos descartables
  scratch/

Orquestación e integración del monorepo
  scripts/
  tests/
  run_f1_94.ps1
  SConstruct

Dependencias externas
  third_party/
```

`game/project.godot` define `game/` como raíz `res://`. Por ello:

```text
game/assets/...  <-> res://assets/...
game/sounds/...  <-> res://sounds/...
```

No se crea un `content/` raíz ni una capa de sincronización adicional.

---

## 4. Mapa de primer nivel

| Ruta actual | Rol actual | Acción | Zona objetivo | Slice |
|---|---|---|---|---|
| `game/` | Runtime Godot, contenido, crates y tooling mezclado | **SPLIT interno** | `game/` + `tools/` + `source-assets/` | Todos |
| `tools/` | Tooling offline parcialmente organizado | **KEEP + REORGANIZE** | `tools/` | ARCH/AUDIO/ENV/VEH/TRACK |
| `blender/` | Fuentes Blender, pipelines, outputs y caches | **SPLIT** | `tools/`, `source-assets/`, `scratch/` | ENV/VEH/TERR/TRACK |
| `assets-lowpoly-python/` | Fuentes, librerías, audio y outputs legacy | **SPLIT** | `source-assets/`, `tools/`, `game/assets` | AUDIO/ENV/VEH |
| `references/` | Referencias y procedencia | **MOVE** | `source-assets/references/` | ENV/VEH |
| `native/` | Fuente C++ GDExtension runtime | **MOVE posterior** | `game/native/` | CLEAN |
| `scripts/` | Build, launch, smoke y orquestación | **KEEP** | `scripts/` | ARCH/CLEAN |
| `tests/` | Integración transversal y fixtures | **KEEP + SPLIT gradual** | `tests/` + tests de dominio | CLEAN |
| `third_party/` | Dependencias externas | **KEEP** | `third_party/` | Ninguno |
| `implementation/` | Implementaciones y assets legacy mixtos | **REVIEW** | Por clasificar | ENV |
| `reports/` | Evidencia, renders y audio de revisión | **REVIEW** | `reports/` o `scratch/` | Cada slice |
| `diagnostics/` | Resultados diagnósticos | **REVIEW** | `scratch/diagnostics/` si son descartables | CLEAN |
| `user/` | Datos locales/usuario | **REVIEW** | Fuera de arquitectura fuente | CLEAN |
| `SConstruct` | Build C++ raíz | **KEEP** mientras compile `native/` | Raíz; reevaluar con `game/native` | CLEAN |
| `run_f1_94.ps1` | Fachada canónica | **KEEP** | Raíz | ARCH |

---

## 5. `game/`: runtime y contenido

### KEEP

| Ruta | Razón |
|---|---|
| `game/project.godot` | Define la raíz runtime |
| `game/scenes/` | Escenas cargadas por el juego |
| `game/scripts/` | Comportamiento y glue Godot |
| `game/crates/` | Motores Rust runtime |
| `game/addons/` | GDExtensions y plugins runtime |
| `game/assets/` | Contenido visual cargable |
| `game/sounds/` | Contenido de audio cargable |
| `game/tests/` | Smoke e integración Godot |
| `game/data/` | Datos runtime; revisar subárboles de autoría por slice |

### SPLIT o MOVE

| Ruta actual | Problema | Destino esperado | Slice |
|---|---|---|---|
| `game/resources/environment/tools/` | Genera contenido dentro del runtime | `tools/assets/environment/` | ENV |
| `game/resources/environment/tests/` | Prueba generadores y outputs mezclados | Tests junto al tooling + integración runtime mínima | ENV |
| `game/resources/environment/recipes/` | Es input de autoría, no runtime puro | `source-assets/environment/recipes/` | ENV |
| `game/resources/environment/palettes/` | Es input editable | `source-assets/environment/palettes/` | ENV |
| `game/resources/environment/manifests/` | Mezcla construcción y runtime | Dividir entre fuentes y metadata runtime necesaria | ENV |
| `game/resources/environment/assets/` | Outputs cargables y manifests de construcción | Outputs a `game/assets/environment/`; inputs fuera de game | ENV |
| `game/tools/` | Contiene validación/runtime tooling mezclado | Clasificar: loader runtime permanece; autoría va a `tools/` | TRACK/CLEAN |
| `game/audio/` | Revisar si es lógica runtime o material de autoría | Runtime queda; autoría va a `tools/audio` | AUDIO |

### Regla

No se mueve una ruta cargada mediante `res://` hasta actualizar y verificar sus
consumidores. La reorganización interna de `game/` ocurre después del cutover
del productor, no antes.

---

## 6. `tools/`: tooling offline

### KEEP y destino

| Ruta actual | Acción | Destino conceptual | Slice |
|---|---|---|---|
| `tools/audio/` | **KEEP + REFACTOR paths** | `tools/audio/` | AUDIO |
| `tools/track_studio/` | **KEEP; reagrupar después** | `tools/track/studio/` | TRACK |
| `tools/vehicle_studio/` | **KEEP; reagrupar después** | `tools/vehicle/studio/` | VEH |
| `tools/asset_pipeline/` | **MOVE interno** | `tools/vehicle/runtime_export/` | VEH |
| `tools/sprites/` | **MOVE interno** | `tools/assets/sprites/` | ENV |
| `tools/background_analysis/` | **REVIEW** | `tools/assets/background/` o `tools/track/background/` | TRACK |
| `tools/physics_diagnostics/` | **KEEP** | Tooling runtime/diagnóstico | CLEAN |
| `tools/validation/` | **SPLIT** | Validación por dominio + integración raíz | CLEAN |
| `tools/cleanup/` | **KEEP** | Mantenimiento del monorepo | CLEAN |
| `tools/agent_validation/` | **REVIEW** | Infraestructura de desarrollo | CLEAN |

No se reorganiza `tools/` en un solo commit. Cada dominio mueve su tooling
cuando sus imports, defaults y tests ya conocen las nuevas rutas.

---

## 7. `blender/`: clasificación transitoria

`blender/` no representa una responsabilidad: mezcla aplicación, pipelines,
fuentes y outputs. Debe desaparecer gradualmente como zona arquitectónica, no
mediante un movimiento masivo.

| Ruta actual | Acción | Destino esperado | Slice |
|---|---|---|---|
| `blender/track_pipeline/` | **SPLIT** | `tools/track/` + `tools/terrain/` | TERR/TRACK |
| `blender/generated/` | **SPLIT/RETIRE** | Preview a `scratch/`; outputs aprobados a `game/assets/` | TERR/TRACK |
| `blender/assets/` | **MOVE** | `source-assets/textures/` y `source-assets/environment/` | ENV/TRACK |
| `blender/vehicle_pipeline/` | **MOVE** | `tools/vehicle/pipeline/` | VEH |
| `blender/vehicle_studio/` | **REVIEW/MOVE** | `tools/vehicle/blender/` si sigue activo | VEH |
| `blender/williams94_wheels_retextured/` | **SPLIT** | Fuentes a `source-assets/vehicles/f1_94/`; outputs aprobados a `game/assets/` | VEH |
| `blender/obj_validator/` | **MOVE** | `tools/vehicle/validation/` o `tools/assets/validation/` | VEH/ENV |
| `blender/analysis_suite/` | **REVIEW/MOVE** | Dominio que realmente consume el análisis | VEH/ENV |
| `blender/vegetation/` | **SPLIT** | `tools/assets/environment/vegetation/` + fuentes | ENV |
| `blender/vegetation_v2_upload_bundle/` | **REVIEW** | Fuente, output aprobado o legacy | ENV |
| `blender/infrastructure/` | **REVIEW** | Tooling o fuente según contenido | ENV |
| `blender/vehicle_pipeline_v2/` | **RETIRE si sigue vacío** | Ninguno | CLEAN |

Los `.blend`, imágenes editables y referencias son fuentes. Los `.py` que
transforman contenido son tooling. Los GLB/PNG generados son preview o contenido
runtime según hayan pasado promoción.

---

## 8. `assets-lowpoly-python/`: clasificación transitoria

Esta ruta mezcla procedencia, fuentes y assets derivados. No se renombra como
unidad.

| Subárbol | Acción | Destino esperado | Slice |
|---|---|---|---|
| `assets-lowpoly-python/sounds/` | **MOVED** | `source-assets/audio/legacy-f1-1998/` | AUDIO-002 |
| `assets-lowpoly-python/vehicles/` | **SPLIT** | Fuentes a `source-assets/vehicles/`; tooling a `tools/vehicle/` | VEH |
| `assets-lowpoly-python/nature/` | **SPLIT** | Fuentes a `source-assets/environment/`; outputs aprobados a `game/assets/environment/` | ENV |
| `assets-lowpoly-python/track_props/` | **SPLIT** | Fuentes a `source-assets/environment/`; outputs a runtime | ENV/TRACK |
| `assets-lowpoly-python/circuit_assets_final/` | **REVIEW** | Distinguir fuente, librería aprobada y legacy | TRACK |
| `assets-lowpoly-python/background/` | **SPLIT** | Fuentes a `source-assets/tracks/` o environment; runtime a `game/assets/` | TRACK |

Cada slice elimina referencias runtime hacia el subárbol migrado antes de
considerarlo terminado.

---

## 9. Fuentes objetivo

Las rutas se crean únicamente cuando un slice migra contenido real:

```text
source-assets/
├── audio/                    AUDIO
├── environment/
│   ├── barriers/             ENV primero
│   ├── vegetation/
│   ├── buildings/
│   └── materials/
├── vehicles/
│   └── f1_94/                VEH primero
├── tracks/
│   └── la_chutana/           TERR/TRACK
├── textures/
└── references/
```

No se crean árboles vacíos completos en ARCH-003.

---

## 10. Formatos objetivo

`formats/` se mantiene vacío salvo README hasta que un slice demuestre varios
consumidores.

Primer candidato confirmado:

```text
formats/audio_bank/
```

porque el banco es consumido por tooling Python, Rust runtime y GDScript.

Candidatos posteriores, no aprobados por anticipado:

```text
formats/vehicle/
formats/terrain/
formats/track/
```

No se mueve un schema solo para llenar la estructura objetivo.

---

## 11. Outputs, reports y temporales

| Tipo | Destino |
|---|---|
| Preview y output intermedio | `scratch/<dominio>/` |
| Contenido visual aprobado | `game/assets/` |
| Audio aprobado | `game/sounds/` |
| Evidencia humana que deba versionarse | `reports/<dominio>/` |
| Diagnóstico descartable | `scratch/diagnostics/` |
| Cache de compilador/editor | Ignorado; nunca es fuente |

Directorios como `.codex-target`, `.codex-native`, `.pytest_cache`, `.ruff_cache`,
`.tmp`, `tmp`, `.godot-user` y caches locales no forman parte de la arquitectura.
Su limpieza se trata por separado y nunca se mezcla con un migration slice.

---

## 12. Reglas de transición

1. No mover una fuente y su consumidor en commits inconexos que dejen el canon
   script roto.
2. No usar `git add -A`; cada slice stagea rutas explícitas.
3. No copiar silenciosamente el mismo archivo a dos fuentes canónicas.
4. Los adaptadores temporales declaran qué slice los elimina.
5. Preview nunca tiene `game/assets` o `game/sounds` como output por defecto.
6. Promote es la única operación autorizada para escribir contenido runtime.
7. Los imports `.godot` y artefactos ignorados no se usan como evidencia de
   que una migración está completa.
8. El movimiento físico ocurre después de identificar todos los lectores y
   escritores del subárbol afectado.
9. `run_f1_94.ps1` debe seguir verde al cerrar cada slice, no necesariamente
   durante un commit intermedio claramente marcado y no integrado.
10. Un fallo conocido no relacionado no autoriza introducir uno nuevo.

---

## 13. Orden de cutover

```text
ARCH-003  raíces mínimas
ARCH-004  protección scratch/runtime
ARCH-005  FAST/FULL
ARCH-006  inventario reproducible de rutas y escrituras
ARCH-007  preview/promote

AUDIO     source-assets/audio -> tools/audio -> game/sounds
ENV       barriers -> tools/assets/environment -> game/assets/environment
VEH       f1_94 -> tools/vehicle -> game/assets/models/vehicles/f1_94
TERR      fuentes topográficas -> tools/terrain -> output simple
TRACK     fuentes pista -> tools/track -> game/assets/generated/tracks
CLEAN     adaptadores, tooling runtime mezclado y legacy confirmado
```

Este orden reduce riesgo: prueba primero una frontera pequeña y con consumidores
claros, y deja los movimientos más acoplados para cuando preview/promote ya sea
un mecanismo estable.

---

## 14. Criterio de cierre de ARCH-002

- La raíz Godot y el contenido `res://` están identificados.
- Cada directorio de primer nivel tiene acción y destino.
- Las rutas mixtas principales tienen estrategia SPLIT.
- Cada movimiento pertenece a un slice futuro.
- Se distinguieron runtime, fuentes, tooling, formatos y temporales.
- No se movió, eliminó ni regeneró ningún archivo.
