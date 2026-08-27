# Blueprint de desacoplamiento indie — Formula90s

> **Estado: EN EJECUCIÓN — Fundaciones y Audio completados.**
> Fecha: 2026-08-26.
> Base: rama `f1-94` @ `d7d8a08fca460341cf1591f508bd58be4c7d1c14`.
> Este documento define la dirección arquitectónica. Cada movimiento de código
> o assets se ejecuta como un backlog item independiente y con staging explícito.
> Siguiente item: **ENV-001 — mapa de migración de barreras**.

---

## 1. Objetivo

Desacoplar Formula90s sin convertir el proyecto en una colección de productos
internos, contratos anticipados o procesos de publicación innecesarios.

La arquitectura debe proteger el runtime y aclarar el ciclo de vida del
contenido, manteniendo corto el flujo diario:

```text
editar -> generar -> abrir en Godot -> probar -> repetir
```

Se introduce una frontera únicamente cuando resuelve un problema real:

- tooling mezclado con runtime;
- fuentes editables mezcladas con contenido cargable;
- herramientas que escriben sobre contenido canónico durante una preview;
- rutas cruzadas difíciles de seguir;
- datos duplicados entre Python, Rust, GDScript y JSON;
- builds que ejecutan DLLs u outputs antiguos;
- Terrain y Track tomando decisiones que pertenecen al otro.

Regla principal:

> **Desacoplar donde acelere el trabajo o evite daño real. No crear fronteras
> por necesidades hipotéticas.**

---

## 2. Principios

1. **Monorepo:** Formula90s permanece en un solo repositorio.
2. **Fail-first:** una preview real ocurre antes del endurecimiento.
3. **Human gate temprano:** primero se decide si el resultado vale la pena;
   después se pagan pruebas, manifests y documentación adicionales.
4. **Dirección simple:** fuentes -> herramientas -> contenido -> runtime.
5. **Promoción explícita:** experimentar y escribir contenido canónico son
   operaciones diferentes.
6. **Formalización bajo demanda:** un formato se estabiliza cuando tiene
   consumidores reales o cuando su ambigüedad ya causa errores.
7. **Dos niveles visibles:** FAST durante desarrollo y FULL al integrar.
8. **SRP práctico:** separar responsabilidades y ciclos de vida, no convertir
   cada carpeta en un producto independiente.
9. **Migración oportunista:** mover dominios completos mientras se trabajan;
   no detener el juego para reorganizar todo el árbol.
10. **Runtime reproducible:** `run_f1_94.ps1` mantiene la protección BUILD/HEAD.

---

## 3. Arquitectura objetivo

La raíz real del proyecto Godot es `game/`; por tanto, el contenido cargable
permanece físicamente dentro de esa raíz. No se introduce un `content/` externo
que requiera sincronización o instalación adicional.

```text
Formula90s/
├── game/                    Proyecto Godot y runtime
│   ├── assets/              Contenido visual cargable por `res://assets/`
│   ├── sounds/              Audio cargable por `res://sounds/`
│   ├── scripts/             Glue y comportamiento Godot
│   ├── scenes/              Escenas runtime
│   ├── crates/              Runtime Rust
│   ├── addons/              GDExtensions/plugins runtime
│   └── tests/               Smoke e integración Godot
├── tools/                   Autoría, transformación y validación offline
│   ├── vehicle/
│   ├── track/
│   ├── terrain/
│   ├── audio/
│   ├── assets/
│   ├── texture/
│   └── common/
├── source-assets/           Fuentes editables y procedencia
│   ├── vehicles/
│   ├── tracks/
│   ├── environment/
│   ├── audio/
│   ├── textures/
│   └── references/
├── formats/                 Formatos compartidos que ya lo necesitan
├── scratch/                 Experimentos y previews descartables
├── docs/
└── run_f1_94.ps1            Entrada canónica
```

Esta es una dirección, no una orden de mudanza masiva. Los directorios legacy
conviven temporalmente hasta que un slice complete productor, output y
consumidor.

---

## 4. Zonas y responsabilidades

### 4.1 `game/`

Contiene todo lo necesario para ejecutar Formula90s:

- proyecto Godot;
- escenas, HUD, cámaras, IA y lógica de carrera;
- físicas Rust y GDExtensions;
- mezcla y reproducción de audio runtime;
- rendering y skybox runtime;
- loaders y validaciones necesarias para cargar contenido;
- contenido aprobado bajo `game/assets` y `game/sounds`.

El runtime puede leer y validar contenido. No debe:

- abrir Blender;
- reparar meshes;
- reducir polígonos;
- generar texturas;
- sintetizar o remasterizar audio offline;
- reconstruir circuitos;
- modificar fuentes editables;
- ejecutar tooling de autoría.

### 4.2 `tools/`

Agrupa herramientas por el trabajo que realizan, no como productos separados.

```text
tools/vehicle   vehículos, geometría, anchors, UV y export
tools/track     Track Studio, spline, pits, barriers, placements y export
tools/terrain   elevación, heightmaps, malla y zonas topográficas
tools/audio     análisis, síntesis offline, loops, remaster y bancos
tools/assets    environment, props, vegetación, edificios y sprites
tools/texture   generación y procesamiento de texturas
tools/common    infraestructura estable realmente compartida
```

Una herramienta puede usar código estable de `tools/common`. Las dependencias
entre herramientas se permiten mientras sean claras y no circulares. Si una
pieza compartida cambia repetidamente por necesidades de un solo dominio, no
pertenece a `common`.

### 4.3 `source-assets/`

Contiene material editable o de procedencia:

- `.blend`, SVG y heightmaps;
- WAV originales;
- imágenes y texturas fuente;
- modelos de alta resolución;
- referencias y licencias;
- datos antes de procesar.

`game/` no depende directamente de esta zona. Las fuentes grandes pueden usar
Git LFS hacia adelante cuando hayan sido clasificadas; reescribir historial
requiere una decisión separada.

### 4.4 Contenido runtime

El contenido aprobado vive en:

```text
game/assets/
game/sounds/
```

Godot lo carga directamente mediante `res://assets/...` y `res://sounds/...`.
No todo archivo necesita manifest:

- un GLB, PNG o WAV independiente puede ser directo;
- un conjunto que debe permanecer coherente usa metadata mínima.

Un manifest se justifica para:

- bancos de audio;
- vehículos multifichero con anchors y configuración;
- pistas con geometría, colisión, checkpoints y placements;
- contenido cuya procedencia o licencia debe conservarse;
- outputs generados que deben validarse como una unidad.

### 4.5 `formats/`

No es una plataforma de contratos. Guarda formatos ya compartidos por dos o
más consumidores importantes.

```text
formats/
├── vehicle/
├── track/
├── terrain/
└── audio_bank/
```

Durante desarrollo, productor y consumidor pueden cambiar en el mismo commit.
Se añade versionado cuando conservar compatibilidad antigua tenga un beneficio
real. No se requieren owners, semver ni suites de conformidad para un formato
que aún cambia diariamente.

### 4.6 `scratch/`

Zona ignorada por Git para:

- previews;
- A/B visuales o de audio;
- escenas aisladas;
- GLB, WAV y texturas temporales;
- outputs intermedios;
- scripts diagnósticos descartables.

No necesita manifests ni estructura rígida. Su única regla obligatoria es:

> **Una ejecución de preview nunca sobrescribe contenido aprobado.**

---

## 5. Flujo de datos

```text
source-assets
      |
      v
    tools
      |
      +------ preview ------> scratch
      |                         |
      |                         v
      |                    HUMAN GATE
      |                         |
      +------ promote <---------+
      |
      v
game/assets + game/sounds
      |
      v
    game
```

### Preview

- Escribe obligatoriamente bajo `scratch/`.
- Ejecuta solo validaciones FAST necesarias para abrir el resultado.
- No requiere hash, manifest definitivo ni compatibilidad histórica.
- Si el resultado se rechaza, se elimina sin trabajo adicional.

### Promote

- Es una acción explícita, nunca un efecto secundario de preview.
- Reconstruye o copia desde inputs aprobados.
- Valida que source y target estén dentro de raíces permitidas.
- Evita instalaciones parciales en bundles multifichero.
- Ejecuta pruebas dirigidas antes de cerrar la integración.

No se introduce package registry interno. Si en el futuro se distribuye
contenido fuera del monorepo, se evaluarán paquetes inmutables y versionados.

---

## 6. Fail-first y human gate

Para cambios visuales, auditivos o jugables:

```text
hacer cambio -> FAST mínimo -> preview real -> HUMAN GATE
                                           |              |
                                        rechazar       aprobar
                                           |              |
                                        descartar       promote
```

El human gate presenta:

1. Qué cambió.
2. Cómo abrir o reproducir el resultado.
3. Comparación antes/después cuando aporte valor.
4. Limitaciones conocidas.
5. Decisión: aceptar o rechazar.

No se necesita un sistema formal de estados. Git, `scratch/`, el comando de
promoción y el contenido runtime expresan el ciclo de vida suficiente.

Si el endurecimiento posterior cambia perceptiblemente el resultado aprobado,
se muestra de nuevo. Cambios internos de rutas, hashes o implementación que no
alteren el producto no requieren otro gate.

---

## 7. Validación FAST y FULL

Solo existen dos niveles visibles en el trabajo diario.

### FAST

Se usa durante desarrollo y migration slices. Selecciona lo afectado:

- compilar el crate o DLL modificado;
- parsear GDScript;
- importar un asset;
- abrir una escena pequeña;
- ejecutar tests del módulo;
- validar un formato solo si cambió.

Debe ser rápido y no convertirse en una checklist manual extensa.

### FULL

Se usa para:

- cerrar un slice de migración;
- integrar una feature grande;
- cambios transversales;
- merges importantes;
- releases.

El punto canónico sigue siendo `run_f1_94.ps1`, que mantiene build,
BUILD/HEAD, smoke principal y launch.

### Fallos preexistentes

- Una regresión nueva y relacionada bloquea integración.
- Un fallo conocido no relacionado se informa, pero no bloquea el slice.
- Un test inestable no cuenta como evidencia y debe quedar identificado.
- La lista de fallos conocidos no puede crecer silenciosamente.

---

## 8. BUILD/HEAD y procedencia

BUILD/HEAD protege el código runtime frente a DLLs y binarios obsoletos.
`run_f1_94.ps1` debe:

1. resolver el HEAD actual;
2. comprobar el build utilizado;
3. recompilar o rechazar cuando no coincidan;
4. lanzar Godot únicamente con runtime válido.

Esto no obliga a tratar cada PNG, GLB o WAV como un paquete versionado. Los
bundles generados pueden conservar hashes o revisión de origen cuando esa
información evita inconsistencias reales.

---

## 9. Fronteras mínimas

1. `game/` no modifica `source-assets/`.
2. `game/` no ejecuta herramientas de autoría.
3. Preview escribe únicamente bajo `scratch/`.
4. Escribir en `game/assets` o `game/sounds` requiere promoción explícita.
5. `tools/` transforma fuentes en contenido runtime.
6. Se eliminan rutas cruzadas hardcodeadas cuando configuración o una ruta
   relativa simple resuelvan el problema.
7. Se evitan dependencias circulares entre herramientas.
8. Un dato de gameplay o runtime no se duplica en Rust, GDScript y JSON si
   puede tener una sola fuente.
9. Guardar y generar siguen siendo acciones diferentes en los editores.
10. `run_f1_94.ps1` sigue siendo la entrada estándar al juego.

Estas reglas pueden automatizarse gradualmente. No se requiere una plataforma
de governance para comenzar la migración.

---

## 10. Terrain y Track

Terrain y Track son capacidades separadas, no productos independientes.

### Terrain

Responsable de:

- elevación y heightmaps;
- malla y colisión topográfica;
- materiales base;
- zonas o máscaras;
- datos geográficos.

Puede producir:

```text
terrain.glb + metadata simple cuando sea necesaria
```

### Track

Responsable de:

- spline y circuito;
- kerbs, pits y checkpoints;
- barriers;
- placements;
- vegetación y edificios;
- decoración;
- skybox y configuración visual authored.

```text
tools/terrain
      |
      v
terrain.glb + datos simples
      |
      v
tools/track
      |
      v
game/assets/generated/tracks/<track>/
```

Terrain puede declarar zonas aptas para vegetación, pero Track decide qué
objetos se colocan y dónde. `skybox-engine` permanece como runtime renderer.

---

## 11. Audio

```text
source-assets/audio
        |
        v
tools/audio
        |
        +---- preview ----> scratch/audio
        |
        +---- promote ----> game/sounds
                                |
                                v
                           game/audio
```

`tools/audio` puede analizar RPM, preparar loops, sintetizar, normalizar,
remasterizar, comparar y generar bancos.

`game/audio` selecciona bandas, hace crossfade, mezcla y reproduce. Cuando
`audio_bank.json` es leído por Rust, Godot y tooling, su definición pertenece a
`formats/audio_bank/`. El mapa de bandas y RPM vive en el banco, no duplicado
en tres implementaciones.

---

## 12. Vehículos y assets visuales

Flujo de vehículo:

```text
source-assets/vehicles/f1_94
        |
        v
tools/vehicle
        |
        +---- preview ----> scratch/vehicle-tests/f1_94
        |
        +---- promote ----> game/assets/models/vehicles/f1_94
```

Los nombres que el runtime utiliza son interfaces reales y deben preservarse:

- wheel y suspension anchors;
- sockets;
- collision identifiers;
- nombres de piezas consumidos por loaders;
- unidades y sistema de coordenadas.

El mismo patrón se aplica a environment, materiales y sprites. Un archivo
independiente no recibe manifest por uniformidad; un bundle coherente sí puede
tenerlo.

---

## 13. Estrategia de migración

No se reorganiza todo de una vez. Cada slice termina un flujo real:

```text
fuente -> herramienta -> preview -> promote -> runtime
```

### Fase A — Fundaciones — COMPLETADA

Completada mediante ARCH-001 a ARCH-007:

1. Blueprint adoptado como dirección única.
2. Mapeo físico documentado.
3. `source-assets/`, `formats/` y `scratch/` creados con alcance mínimo.
4. Preview protegido frente a escrituras runtime.
5. FAST y FULL disponibles.
6. Inventario reproducible de escrituras y rutas cruzadas disponible.
7. Preview y promote explícitos disponibles.

### Fase B — Audio — COMPLETADA Y APROBADA

Completada mediante AUDIO-001 a AUDIO-010:

1. Fuentes originales separadas de los outputs runtime.
2. `tools/audio` adaptado a las nuevas rutas y límites de escritura.
3. Previews y candidatos aislados en `scratch/audio`.
4. Contrato mínimo del banco compartido por Python y Rust.
5. Bandas, RPM y metadata de reproducción sin tablas runtime duplicadas.
6. Banco canónico promovido a `game/sounds` mediante promote explícito.
7. Implementación GDScript paralela retirada; audio runtime bajo F90Core/Rust.
8. Human gate de sonido aprobado el 2026-08-26.

### Fase C — Environment — SIGUIENTE

Se migra una sola familia primero: barreras. No se abre una migración masiva de
todo Environment.

1. **ENV-001:** mapear fuentes, generadores, outputs, lectores y escrituras de
   barreras sin mover archivos.
2. **ENV-002:** separar las fuentes editables bajo `source-assets/environment/`.
3. **ENV-003:** consolidar el tooling de barreras fuera de `game/`.
4. **ENV-004:** producir candidatos solamente bajo `scratch/environment/`.
5. **ENV-005:** usar un manifest mínimo si el bundle multifichero lo requiere.
6. **ENV-006:** promover explícitamente a runtime y retirar writers directos.
7. **ENV-007:** ejecutar FAST proporcional y un único human gate visual.

### Fase D — Vehículos — PENDIENTE

1. Migrar fuentes y tooling del F1-94.
2. Estabilizar las interfaces runtime reales.
3. Promover outputs bajo `game/assets/models/vehicles/f1_94`.
4. Eliminar referencias runtime hacia fuentes legacy.

### Fase E — Terrain y Track — PENDIENTE

1. Consolidar `tools/terrain` y su output simple.
2. Mover placements a Track.
3. Consolidar Track Studio y exportadores en `tools/track`.
4. Separar fuentes SVG de pistas runtime.
5. Eliminar escrituras destructivas durante preview.

Deuda confirmada: `run_track_pipeline.ps1` todavía genera y publica directamente
en `game/assets/generated/tracks/`. Su validación BUILD/HEAD permanece activa;
la migración futura debe cambiar el destino de preview, no relajar esa paridad.

### Fase F — Limpieza — PENDIENTE

Solo después de cortar dependencias reales:

- retirar adaptadores temporales;
- sacar generadores restantes de `game/`;
- consolidar utilidades genuinas en `tools/common`;
- archivar validadores y documentación obsoletos;
- evaluar el remanente de `assets-lowpoly-python` y `blender/`.

### Backlog priorizado vigente

| Prioridad | Item | Resultado de arquitectura | Gate humano |
|---:|---|---|---|
| 1 | **ENV-001** | Mapa ejecutable de la familia barreras | Ninguno; solo revisión del mapa |
| 2 | **ENV-002–ENV-007** | Barreras separadas de fuente a runtime | Una revisión visual final |
| 3 | **VEH-001** | Mapa de migración del F1-94 | Ninguno |
| 4 | **VEH-002+** | Flujo desacoplado de assets del vehículo | Una revisión visual final |
| 5 | **TERR-001 / TRACK-001** | Frontera y mapas separados de Terrain y Track | Ninguno |
| 6 | **TERR/TRACK cutover** | Preview sin publicación runtime destructiva | Una revisión jugable final |
| 7 | **CLEAN-001** | Retiro de adaptadores y legacy ya desconectado | Revisión de alcance |

El inventario arquitectónico global es evidencia para seleccionar slices, no un
backlog que deba vaciarse antes de avanzar. Cada item corrige únicamente sus
rutas y consumidores reales.

---

## 14. Definición práctica de desacoplamiento

Una parte está suficientemente desacoplada cuando:

- puede cambiar sin reescribir partes no relacionadas;
- sus inputs y outputs son fáciles de localizar;
- no modifica archivos ajenos de manera oculta;
- puede previsualizarse de forma aislada;
- el runtime no depende de tooling de autoría;
- no duplica datos compartidos innecesariamente;
- puede integrarse mediante FAST y cerrarse mediante FULL.

No necesita convertirse en un producto independiente.

---

## 15. Qué no se implementa ahora

- Divisiones raíz `f90-*`.
- Microservicios o repositorios separados.
- Package registry interno.
- Paquetes inmutables obligatorios para cada asset.
- SHA-256 obligatorio durante preview.
- Versionado semántico para cada formato.
- Owners obligatorios por schema.
- CI independiente por división.
- Estados DRAFT/PREVIEW/APPROVED/PROMOTED.
- Varias aprobaciones humanas para el mismo resultado.
- Capas de compatibilidad sin consumidores reales.
- Reescritura inmediata del historial Git para LFS.

Estas capacidades podrán introducirse si aparecen distribución externa,
múltiples equipos, releases de tooling independientes o incompatibilidades que
justifiquen su coste.

---

## 16. Decisiones registradas

| # | Decisión |
|---|---|
| D1 | Formula90s permanece como monorepo |
| D2 | No se crean divisiones raíz `f90-*` |
| D3 | Tooling y runtime sí se separan |
| D4 | Fuentes editables y contenido runtime sí se separan |
| D5 | `game/assets` y `game/sounds` siguen siendo contenido runtime por estar bajo `res://` |
| D6 | `scratch/` contiene previews y nunca sobrescribe contenido aprobado |
| D7 | Preview y promote son operaciones distintas |
| D8 | Solo FAST y FULL son niveles visibles de validación |
| D9 | El human gate ocurre después de preview y antes del endurecimiento |
| D10 | Los formatos se estabilizan al tener consumidores reales |
| D11 | Terrain y Track son capacidades separadas, no productos independientes |
| D12 | Track decide placements; Terrain produce topografía y zonas |
| D13 | `run_f1_94.ps1` mantiene BUILD/HEAD y sigue siendo canónico |
| D14 | La migración se ejecuta por dominios completos y sin detener features |
| D15 | Manifests se usan para bundles coherentes, no para cada archivo |
| D16 | Git LFS se adopta hacia adelante después de clasificar fuentes |

---

## 17. Principio final

Formula90s es un juego indie en desarrollo, no una plataforma interna.

La arquitectura debe ayudar a:

```text
probar más
romper menos
cambiar rápido
entender dónde vive cada cosa
```

Si una capa arquitectónica añade más trabajo del que elimina, todavía no hace
falta.
