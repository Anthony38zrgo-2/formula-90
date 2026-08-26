# Blueprint de Desacoplamiento — Formula90s

> **Estado: PROPUESTO (pendiente de aprobación humana).**
> Fecha: 2026-08-26. Procedencia registrada: rama `f1-94` @ `145919a`, árbol con 43
> entradas sucias inventariadas antes de redactar este documento. Este blueprint
> es solo diseño/auditoría: **no mueve código**. La ejecución requiere aprobación
> explícita fase por fase (ver §10).

---

## 1. Motivación

El repositorio actual cumple tres responsabilidades distintas en un solo árbol:
fábrica de assets, autoría/compilación de pistas y runtime del juego. La
evidencia del desacople pendiente:

1. **Peso dominado por assets, no por código.** `blender/assets/` contiene
   ~15.163 archivos de fuentes de texturas (biomas de La Chutana con provenance
   SHA-256). El trabajo de pipeline y el trabajo de gameplay compiten en el
   mismo historial git (commits `build: republish la chutana runtime...`
   mezclados con feats de física).
2. **Acoplamiento por rutas frágiles.** `_REPO_ROOT = Path(__file__).resolve().parents[2]`
   en `build_svg_track.py:63`, `compile_svg_track.py:36`, `import_la_chutana.py:62`,
   `validate_godot_load.py:47`, entre otros. `blender/track_pipeline/configs/la_chutana.json`
   mezcla en un solo archivo rutas de `blender/`, `game/assets/`, `game/resources/`
   y `assets-lowpoly-python/` (líneas 3-5, 154-158, 226-234, 297, 319-331).
   Mover cualquier directorio rompe scripts en silencio.
3. **Duplicación por falta de fronteras.** El sanitizador SVG está implementado
   3 veces: `blender/track_pipeline/svg_sanitizer.py` (Python),
   `tools/track_studio/crates/track-import-svg` (Rust),
   `tools/vehicle_studio/app/src/svgSanitizer.ts` (TypeScript). La generación
   de terreno/vegetación está duplicada Python↔Rust con paridad pendiente.
4. **El juego quiere ser árbitro, no fábrica.** `game/tools/validate_generated_track.gd`
   ya es la "única fuente de verdad" del gate de pistas y `f1_94_physics.json`
   usa `deny_unknown_fields`. La dirección natural es que el runtime **solo
   valide manifiestos** al ingestar coches/mapas nuevos.
5. **La dirección ya estaba anunciada.** `docs/roadmap.md` (§ "Editor de
   escenario/pistas y pipelines independientes", PRÓXIMA PRIORIDAD) pide
   separar el pipeline de producción de assets del pipeline de autoría, un
   Asset Registry único por IDs semánticos y Blender reducido a compilador
   headless. Vehicle Studio ya ejecutó este patrón (documento versionado +
   BuildIR + Vue + materializador). Este blueprint generaliza el patrón.

**Principio rector: SRP por división.** Cada `f90-*` tiene una única
responsabilidad y una única razón para cambiar.

---

## 2. Arquitectura objetivo

**Repositorio único** (el actual Formula90s) con **divisiones claras entre
subrepos**. Cada división es autocontenida (propio README, propios tests, propio
tooling) y las fronteras entre divisiones son verificables, no convencionales.

```
Formula90s/
├── f90-contracts/        Contratos versionados: schemas JSON + fixtures de conformancia
├── f90-topo-builder/     Terreno: SVG semántico + JSON → terrain.glb + placement.json
├── f90-assets-factory/   Assets deterministas + remodelación (fuentes en Git LFS)
├── f90-track-builder/    Circuito: compone SVG+JSON+terreno+skybox+geografía
├── f90-game/             Runtime Godot/C++/Rust + validador de manifiestos
├── docs/                 Documentación transversal (este documento, ADRs, PROJECT_STATE)
├── AGENTS.md / README.md / ROADMAP.md / PROJECT_STATE.md
└── run_f1_94.ps1         Canon script (sin cambios; ver §9)
```

### Responsabilidad única por división

| División | Stack | Responsabilidad (una sola) | NO hace |
|---|---|---|---|
| `f90-contracts` | JSON Schema + fixtures | Definir y versionar los contratos entre divisiones | No contiene lógica de negocio |
| `f90-topo-builder` | Python + Blender + editor Vue auditor | Generar el 3D del terreno (montañas) desde SVG semántico + JSON | No compone el circuito, no produce assets |
| `f90-assets-factory` | Python + Blender | Crear assets deterministas y remodelar existentes, cumpliendo el contrato de coches/pistas | No consume SVG de pistas, no decide colocación |
| `f90-track-builder` | Python + Blender + Track Studio (Rust/Tauri) | Generar el paquete de pista completo: circuito + skybox + geografía | No genera terreno base (lo consume), no fabrica assets |
| `f90-game` | Godot 4.7.1 + C++20 GDExtension + Rust | Runtime del juego y **validación de manifiestos** al ingestar | No genera ni remodela assets |

### Relación con Track Studio

Track Studio (Rust/Tauri, `tools/track_studio`, plan TS-000..TS-180 vigente en
`docs/track-studio/migration-plan.md`) **convive**: se convierte en el editor
oficial de `f90-track-builder`. La imagen de arquitectura define
responsabilidades, no stack. El plan de migración TS se mantiene como roadmap
interno de la división; `f90-track-builder` hereda sus contratos
(TrackDocument v1 / BuildIR v1 de `docs/track-studio/contracts-v0.md`) vía
`f90-contracts`.

---

## 3. Flujo de datos

```
                    ┌────────────────────┐
                    │   f90-contracts    │  schemas versionados (única fuente canónica)
                    └─────────┬──────────┘
              consume │        │        │ consume
        ┌─────────────▼──┐  │   ┌────▼──────────┐
        │ f90-topo-      │  │   │ f90-assets-   │
        │ builder        │  │   │ factory       │
        │ (SVG+JSON →    │  │   │ (assets +     │
        │  terreno)      │  │   │  bancos textura)│
        └───────┬────────┘  │   └──┬────────┬───┘
                │           │      │        │
   terrain.glb + placement.json │        │ packs de coches + manifiesto
   (terrain_package_v1)         │        │ bancos de textura + manifiesto
                │               │        │ (asset_pack_manifest_v1)
        ┌───────▼───────────────▼──┐     │
        │ f90-track-builder        │◄────┘  consume por manifiesto (ID semántico)
        │ (compone circuito +      │
        │  skybox + geografía)     │
        └───────┬──────────────────┘
                │ track_package_v1 + manifiesto
        ┌───────▼──────────────────┐
        │ f90-game                 │
        │ VALIDADOR de manifiestos │──► acepta / rechaza con diagnóstico
        └──────────────────────────┘
```

Reglas del flujo:

- `f90-topo-builder` y `f90-track-builder` **consumen por manifiesto** lo que
  produce `f90-assets-factory` (bancos de textura, props, vegetación, coches),
  referenciado por ID semántico, nunca por ruta de archivo.
- `f90-game` es la única puerta de entrada al runtime: valida esquema,
  provenance SHA-256 y paridad BUILD/HEAD antes de aceptar un paquete.
- `Save` está separado de `Validate & Build`: guardar solo persiste el SVG;
  compilar es una acción humana explícita (ya definido en `docs/roadmap.md`).

---

## 4. f90-contracts

División autocontenida que centraliza **todos** los schemas. Objetivo: que no
exista discrepancia sobre qué versión del contrato es canónica.

### Política de versionado canónico

1. `f90-contracts` es la **única ubicación** permitida de schemas. Prohibido
   vendorar copias en otras divisiones (hoy existen 3 sanitizadores SVG; el
   perfil SVG restringido pasa a ser un schema + fixtures aquí).
2. Cada schema lleva versión semántica propia (`*_v1.schema.json`). Cambios
   incompatibles = nueva versión mayor, la anterior se congela, nunca se edita.
3. Cada división declara en su manifiesto de configuración la versión pineada
   de cada contrato que consume. El validador de `f90-game` rechaza paquetes
   cuya versión de contrato no esté soportada.
4. Cada schema incluye fixtures de conformancia (ejemplo válido + inválidos)
   ejecutables por pytest/cargo desde cualquier división.

### Inventario de schemas (existentes a formalizar → migrar aquí)

| Contrato | Hoy vive en | Estado |
|---|---|---|
| `vehicle_document_v1.schema.json` | `tools/vehicle_studio/schemas/` | Mover + congelar v1 |
| `vehicle_build_ir_v1.schema.json` | `tools/vehicle_studio/schemas/` | Mover + congelar v1 |
| Manifiesto runtime de vehículo | emitido por `tools/asset_pipeline/generate_f1_94_runtime.py:503` | Extraer schema formal |
| Track revision manifest (hash-chained) | `game/data/tracks_revisions/<track>/revisions/<sha>/manifest.json` | Extraer schema formal |
| Asset Registry (IDs semánticos) | `blender/track_pipeline/configs/asset_registry.json` | Extraer schema formal |
| Perfil SVG restringido | duplicado en Python/Rust/TS | Unificar: schema + fixtures |
| TrackDocument v1 / BuildIR v1 / Diagnostic v1 | `docs/track-studio/contracts-v0.md` (prosa) | Formalizar como schemas |
| Barrier/building construction manifests | `game/resources/environment/manifests/` | Extraer schema formal |
| Bank manifest de audio | `game/sounds/banks/v10_vehicle/bank_manifest.json` ↔ `tools/audio/bank_manifest.py` | Extraer schema formal |

### Schemas nuevos requeridos por la imagen de arquitectura

| Contrato | Productor → Consumidor | Contenido |
|---|---|---|
| `terrain_package_v1` | topo-builder → track-builder | `terrain.glb` (heightfield) + `placement.json` (biomas, colocaciones, metadatos de winding Godot `(x,z,h)` vs Blender `(x,-z,h)`) |
| `track_package_v1` | track-builder → f90-game | Circuito GLB + skybox + geografía + manifiesto con provenance SHA-256 |
| `asset_pack_manifest_v1` | assets-factory → topo/track/game | Pack de assets con IDs semánticos, hashes y versión de contrato |

---

## 5. Auditoría KEEP / MOVE / REFACTOR / DEPRECATE

### → `f90-assets-factory`

| Elemento actual | Acción |
|---|---|
| `blender/assets/` (~15.163 archivos: texture_sources por bioma, buildings, ps1-monacogp) | **MOVE + Git LFS** |
| `blender/vehicle_pipeline/`, `blender/vehicle_studio/`, `blender/williams94_wheels_retextured/` | **MOVE** |
| `blender/obj_validator/`, `blender/analysis_suite/` | **MOVE** (QA de assets) |
| `blender/vegetation/`, `blender/vegetation_v2_upload_bundle/`, `implementation/` | **MOVE** |
| `assets-lowpoly-python/` | **MOVE** |
| `tools/asset_pipeline/` (generación runtime de coches) | **MOVE** |
| `tools/audio/` (bank_generator, bank_validator, promote) | **MOVE** |
| `tools/sprites/` | **MOVE** |
| `references/` (fuentes intactas + auditorías de procedencia) | **MOVE** (decisión registrada §9; el juego solo retiene hashes en manifiestos) |
| `blender/track_pipeline/texture_forge.py` | **MOVE + REFACTOR** (produce bancos de textura que topo/track consumen por manifiesto) |
| `game/resources/environment/` (manifests + generadores de barriers/buildings) | **MOVE** los manifests+generators; `f90-game` conserva solo los GLB finales validados |

### → `f90-topo-builder` (extracción nueva, la más delicada)

| Elemento actual | Acción |
|---|---|
| `blender/track_pipeline/terrain_grid.py` | **MOVE + REFACTOR**: deja de escribir dentro del pipeline y pasa a emitir `terrain_package_v1` |
| Parte terrain de `blender/track_pipeline/blender_backend/` (road/terrain builders → solo terrain) | **MOVE + REFACTOR** |
| Editor Vue auditor de terreno (imagen: "editor-auditor visual en Vue") | **NUEVO** (punto abierto §11.1) |

### → `f90-track-builder`

| Elemento actual | Acción |
|---|---|
| `blender/track_pipeline/` restante (svg_sanitizer, svg_normalizer, build_svg_track, build_normalized_track_blender, asset_registry, curb_manifest, colocación de vegetación, `manifests/`, `configs/`, `layouts/`, `data/`) | **MOVE** |
| `tools/track_studio/` (workspace Rust de 10 crates + shell Tauri + frontend TS) | **MOVE** (editor oficial; hereda el plan TS-000..TS-180) |
| `blender/track_pipeline/authoring/` (server Python + editor Vue 3/SVG.js) | **MOVE** y **DEPRECATE a paridad** de Track Studio (equivale a TS-180) |

### → `f90-game` (KEEP + reducción)

| Elemento actual | Acción |
|---|---|
| `game/` runtime (scripts GDScript, scenes, `game/crates/` Rust, addons, `game/data/`, `game/sounds/`) | **KEEP** |
| `native/` (C++20 GDExtension), `third_party/godot-cpp` (submódulo pineado), `SConstruct`, `build_profile.json/.py` | **KEEP** (mueven a `f90-game/`) |
| `scripts/` (bootstrap/build/test/run) | **KEEP** (mueven a `f90-game/`; `run_f1_94.ps1` permanece en raíz, ver §9) |
| `game/tools/validate_generated_track.gd` + `tools/validation/` (C++) | **KEEP + REFACTOR**: núcleo del módulo validador de manifiestos (§6) |
| `tests/` (fixtures, smoke) | **KEEP** |

### DEPRECATE (en la fase de slim-down, no antes)

| Elemento | Razón |
|---|---|
| `game/core/`, `game/sim/`, `game/physics/engine/` (andamios vacíos) | El código real vive en `game/crates/` |
| `blender/vehicle_pipeline_v2/` (vacío) | Andamio abandonado |
| `game/addons/formula90s/scripts/*.uid` huérfanos | Restos de la mudanza a `game/scripts/` |
| `*.obj` en raíz (~60 MB de artefactos de build) | Nunca debieron committearse |
| `context.md` | Postmortem de la era GEVP; referencia escenas inexistentes. Archivar en `docs/ai/` |
| `rename_manifest.csv` | Legacy |
| `.codex_tmp/`, `.codex-target/`, `.tmp/`, `tmp/` (contenido) | Cachés; limpiar y gitignore |

---

## 6. Validador de manifiestos en f90-game

`f90-game` incorpora un módulo `manifest-validator` (crate Rust + gate Godot
existente) que al ingestar cualquier paquete nuevo (coche o pista) valida:

1. **Esquema**: conformancia contra la versión pineada del schema en
   `f90-contracts` (`deny_unknown_fields` en todo contrato).
2. **Provenance**: SHA-256 de cada artefacto coincide con el manifiesto.
3. **Paridad BUILD/HEAD**: el runtime rechaza binarios cuyo BUILD no coincida
   con HEAD (protocolo AGENTS.md ya lo exige para lanzamiento; se extiende a
   la ingesta).
4. **IDs semánticos**: todo asset referenciado existe en el Asset Registry con
   la versión de contrato declarada.

Resultado: aceptación atómica o rechazo con diagnóstico accionable. El juego
deja de conocer procesos de generación; solo contratos.

---

## 7. Reglas de frontera (verificables)

1. **Prohibido el path arithmetic cruzado**: ningún script puede resolver
   rutas fuera de su división con `Path(__file__).parents[N]`. Cada división
   resuelve rutas relativas a su propia raíz.
2. **Prohibidas las escrituras directas entre divisiones**. Hoy
   `configs/la_chutana.json:5` declara `"runtime_dir": "game/assets/..."` y
   `build_jordan_tire_barrier_asset.py:121` escribe en `game/assets/trackside`.
   Sustituto: publicación de paquetes + manifiesto; el validador de `f90-game`
   es el único que "toca" el runtime.
3. **IDs semánticos, nunca rutas de archivo**, para referenciar assets entre
   divisiones (ya exigido por `docs/roadmap.md` para el Asset Registry).
4. **f90-contracts es la única ubicación de schemas**; prohibido duplicar
   lógica de contrato en otras divisiones (fin de los 3 sanitizadores SVG: el
   perfil vive aquí como schema + fixtures; cada lenguaje implementa contra
   los fixtures dorados).
5. **Binarios solo en f90-assets-factory** (Git LFS). `f90-game` recibe
   artefactos validados publicados, no fuentes de texturas.
6. **Cada división con README, tests y tooling propios**; las pruebas de una
   división no leen archivos de otra (hoy `tools/vehicle_studio/tests` y
   `tests/test_full_barrier_migration.py` leen `game/` directamente: pasan a
   consumir fixtures de `f90-contracts`).
7. **Save separado de Validate & Build** en todos los editores.
8. **CI por división** (punto abierto §11.3) con gate de fronteras: un check
   que fallen si aparece un `parents[N]` cruzado o una ruta `f90-*` hardcodeada
   fuera de su división.

---

## 8. Convención de nombres

Divisiones del repositorio único: `f90-<modulo>`.

- `f90-game`
- `f90-track-builder`
- `f90-assets-factory`
- `f90-topo-builder`
- `f90-contracts`

---

## 9. Decisiones registradas (2026-08-26)

| # | Decisión | Rationale |
|---|---|---|
| D1 | **Repositorio único** (Formula90s actual) con divisiones `f90-*` claramente separadas, en lugar de 4 repos git independientes | Simplifica la operativa actual; las fronteras se vuelven contratos + reglas verificables |
| D2 | **Track Studio convive**: Rust/Tauri es el editor de `f90-track-builder`; la imagen define responsabilidades, no stack | Reaprovecha el plan TS-000..TS-180 ya en curso |
| D3 | **f90-game = runtime + validador only**: todo pipeline (`blender/`, `tools/`) migra fuera de la división | El juego deja de ser fábrica y es árbitro de contratos |
| D4 | **Contratos centralizados en f90-contracts**, división propia, única fuente canónica de schemas, consumo por versión pineada; prohibido vendorar | Cero discrepancia sobre la versión canónica |
| D5 | **Interfaz Topo→Track**: `terrain_package_v1` (terrain.glb + placement.json con biomas/colocaciones) | Permite SRP real entre terreno y circuito |
| D6 | **topo/track consumen por manifiesto lo que produce assets-factory** (incluido `texture_forge.py`, que migra a assets-factory) | IDs semánticos; sin rutas cruzadas |
| D7 | **`references/` migra a f90-assets-factory** | La fuente intacta vive junto a quien remodela; el juego solo retiene hashes |
| D8 | **Assets fuente con Git LFS** en `f90-assets-factory` | ~15k archivos de texturas no deben pesar en el historial del runtime |
| D9 | **Estrategia: solo diseño/auditoría por ahora**; ejecución por fases con aprobación humana | Protocolo AGENTS.md: no mezclar, commits atómicos |
| D10 | **`run_f1_94.ps1` permanece en la raíz** como canon script | No romper AGENTS.md; la paridad BUILD/HEAD se mantiene en cada fase |

---

## 10. Fases de migración (especificación — NO ejecutar sin aprobación)

Cada fase: commits atómicos con staging explícito, sin mezclar cambios, y
`run_f1_94.ps1` verde al cierre. Antes/después de cada fase se corren las
pruebas golden de paridad.

- **Fase 0 — f90-contracts.** Crear la división, mover/formalizar los schemas
  del inventario (§4), publicar tags `contracts-vX.Y.Z`, fixtures de
  conformancia. No toca pipelines. Riesgo: bajo.
- **Fase 1 — f90-assets-factory.** Mover assets + vehicle pipeline + audio +
  sprites + references (D7) + texture_forge (D6). Activar Git LFS (D8).
  Redirigir consumidores vía manifiestos. Riesgo: medio (es la mudanza grande).
- **Fase 2 — f90-topo-builder.** Extraer `terrain_grid.py` + backend terrain;
  definir e implementar `terrain_package_v1`; pruebas golden de paridad contra
  la salida actual de La Chutana. Riesgo: medio-alto (extracción de interfaz).
- **Fase 3 — f90-track-builder.** Mover el resto de `track_pipeline` +
  `tools/track_studio`; consumir `terrain_package_v1` y bancos de assets por
  manifiesto; retirar `authoring/server.py` a paridad Track Studio (TS-180).
  Riesgo: medio.
- **Fase 4 — f90-game slim-down.** Módulo `manifest-validator` operativo (§6),
  mover runtime a `f90-game/`, ejecutar DEPRECATE (§5), actualizar README/
  PROJECT_STATE/ROADMAP. Riesgo: bajo-medio.

Criterio de done por fase: la división tiene README propio, sus tests pasan
sin leer otras divisiones, y el canon script sigue verde.

---

## 11. Puntos abiertos

1. **Editor Vue de f90-topo-builder**: ¿aplicación propia (como la imagen
   sugiere) o modo "Terrain" dentro de Track Studio (el roadmap actual ya
   contempla modos Objects y Terrain sobre el mismo documento)? Pendiente de
   decisión.
2. **Skybox y geografía**: la imagen asigna skybox a track-builder, pero el
   runtime tiene `skybox-engine` (crate Rust). Falta definir qué parte del
   skybox es asset (produce track-builder) y qué parte es runtime (queda en
   f90-game).
3. **CI por división**: qué runners (pytest / cargo test / Godot headless) y
   qué gate de fronteras automatizado (§7.8).
4. **`game/resources/environment/`**: confirmar que manifests+generators migran
   a assets-factory y `f90-game` retiene solo GLB finales validados (asumido
   en §5, requiere verificación de que nada en runtime lee los manifests
   directamente).
5. **Historial git**: las fases usan `git mv` para preservar blame; confirmar
   si se exige historia completa por archivo o basta la del repo único.
