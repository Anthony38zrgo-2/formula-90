# Resolved Incidents (Postmortems)

Repositorio de conocimiento técnico sobre bugs graves, esotéricos o fuertemente arraigados en el engine, ya resueltos.
Evita repetir errores leyendo estos incidentes.

---

## Incident 001: Pérdida Silenciosa de Enlaces a Nodos en Godot 4 (node_paths)

### Symptoms
Scripts como el `VehicleController` o el HUD dejaban de recibir inputs, los sistemas de ayudas devolvían Null Exceptions en runtime, o fallaban referenciando el `DrivingAids`. El árbol parecía correcto a simple vista en el IDE.

### Root cause
En Godot 4, cuando exportas una variable de tipo Node (`@export var my_node: Node`) en un script, el motor de escenas requiere un atributo especial de metadatos en formato texto dentro del `.tscn`: `node_paths=PackedStringArray("my_node")`. 

### Failed approaches
- Añadir getters/setters redundantes.
- Intentar usar `get_node()` en `_ready()` parcheando el código.
- Asumir que la inyección de dependencias estaba mal diseñada.

### Correct fix
Restaurar explícitamente `node_paths=PackedStringArray(...)` en la declaración del nodo principal del archivo `.tscn` cuando se hace un diff o una modificación del texto crudo de la escena.

### Lesson
Los agentes AI tienen la costumbre de regenerar bloques de código `.tscn` basándose en el script original, olvidando los atributos ocultos de metadata inyectados por el inspector de Godot. **Cualquier edición a un `.tscn` exportando nodos exige validación del array `node_paths`.**

---

## Incident 002: Jordan 197 visual invertido respecto a GEVP (J197-VIS-001)

### Symptoms

En la cámara trasera se veía el morro del Jordan 197. Las ruedas anchas parecían
estar junto al morro visual y las estrechas junto a la zaga. La firma ya estaba
presente a `0 km/h` y en neutral.

### Root cause

El GLB canónico usa frontal `+Z`; GEVP, los RayCast y la cámara usan frontal
`-Z`. El generador y el prompt asumían que Godot haría esa conversión semántica,
pero la escena instanciaba `ChassisVisual` con basis identidad. El smoke sólo
verificaba nodos, scripts, superficies y materiales.

### Failed or misleading signals

- Investigar física/suspensión habría sido incorrecto: el fallo existía en reposo.
- Dos smokes terminaron con `0xC0000005`, pero Godot no había llegado a ejecutar
  las aserciones: `user://logs` no era escribible para el usuario sandbox.
- Un `Get-FileHash` con acceso denegado produjo `$null`; eso no era un mismatch.
- `agentdb` no estaba disponible, por lo que la consulta histórica quedó
  indisponible, no “sin resultados”.

### Correct fix

- Mantener el asset en frontal `+Z`.
- Declarar `coordinate_contract` con runtime `-Z` y escena visual como dueño.
- Aplicar yaw 180 a `ChassisVisual`, preservando offset, RayCast y física.
- Exigir en el smoke `visual_forward dot physical_forward >= 0.99`, ejes dentro
  de 1 cm y recursos delantero/trasero correctos.
- Ejecutar Godot con `user://` escribible; crash previo a assertions es
  `INCONCLUSIVE`.

### Validation

`dot -1 -> +1`; smokes Jordan 197, Jordan 191 y bootstrap/HUD PASS; importación,
hashes y apertura headless PASS; captura visual PASS.

### Full retrospective

`docs/troubleshooting/jordan-197-orientation-retrospective.md`

---

## Incident 003: AgentDB rejected by sandbox permissions

### Symptoms

The AgentDB release binary existed, but agent bootstrap reported
`agentdb no disponible`. Direct access returned `unable to open database file`,
and the automatic Cargo fallback failed on
`target/release/.cargo-lock: Access denied`.

### Root cause

The sandbox could read `.agents` but could not write the SQLite database or its
WAL/journal files under `.agents/data`. The resolver treated the valid binary as
unhealthy because its probe could not open the database, then attempted a build
in a protected Cargo target directory.

### Correct fix

Use the existing release binary with an execution context that has write access
to the configured AgentDB database, or repair permissions for the actual
execution identity. Run preflight, health and smoke validation before resuming
any feature work. An empty problem result is meaningful only when the lookup
returns successfully in indexed mode.

### Permanent guardrail

AgentDB availability is a hard prerequisite. If an agent cannot access it, stop
the task and repair the runtime/database access first; do not continue with
vehicle, physics or gameplay changes and do not rebuild blindly.

---

## Incident 004: Jordan 197 faceted lighting while moving (J197-VIS-002)

### Symptoms

The painted body showed large polygon-shaped dark patches. Their intensity
changed with light direction and vehicle movement, and automatic LOD changes
made the transition more conspicuous.

### Root cause

The canonical source and all five Jordan runtime GLBs omitted the glTF `NORMAL`
attribute. The generic mesh analyzer hid this because Trimesh calculates normals
when loading. Raw GLB inspection showed zero primitives with stored normals, so
Godot correctly fell back to flat face normals.

### Correct fix

- Append deterministic smooth normals with a 45-degree crease while smoothing
  coincident UV-seam vertices.
- Preserve every pre-existing GLB buffer and all geometry, UV, material, image,
  node, and transform data.
- Export split runtime scenes with Trimesh `include_normals=True`.
- Reject source or runtime publication when any triangle primitive lacks normals.

### Validation

All 79 source primitives, 30 chassis primitives, and 10 primitives in each wheel
contain `NORMAL`. Source/runtime assembly equivalence passed with a maximum node
geometry error of `0.000089 mm`. The isolated Godot assembly smoke passed with
all four wheels contacting the surface and hub errors below `0.009 mm`.

---

## Incident 005: PowerShell launcher and protected F1-94 assets (F194-OPS-001)

### Symptoms

The user could not launch `scripts/run_f1-94.ps1` by using Explorer. A direct
PowerShell invocation then failed at `Get-Content` with `Access denied` for
`game/assets/models/vehicles/f1_94/vehicle_runtime_manifest.json`.

### Root cause

Two independent Windows configuration facts produced the same user-visible
result:

- `Microsoft.PowerShellScript.1/Open` was associated with Notepad and this
  profile had no `RunWithPowerShell` shell verb. Opening a `.ps1` was therefore
  editing it, not executing it.
- The F1-94 runtime files had protected ACLs. The vehicle directory looked
  accessible, but child files did not inherit its permissions, so the interactive
  user could not read the manifest or GLBs.

### Correct fix

- Invoke the script explicitly with
  `powershell.exe -NoProfile -File D:\Formula90s\scripts\run_f1-94.ps1`.
  A project-local launcher is preferable when double-click execution is required;
  do not depend on the generic `.ps1` Open association.
- For the exact `game/assets/models/vehicles/f1_94` tree only, elevate to take
  ownership, enable ACL inheritance and grant the interactive user `(OI)(CI)M`
  recursively. This preserves the project scope and avoids broad ACL changes.
- Check execution policy independently of shell association and ACLs.

### Validation

All 24 F1-94 runtime files opened for read after the ACL repair.
`powershell.exe -NoProfile -File ...\run_f1-94.ps1 -Smoke` exited `0` and
reported `[PASS] F1-94 loads on La Chutana with 4/4 contacts, HUD, speed gauge,
and live minimap.` Existing Jordan 197 resource errors emitted during the
whole-project import were unrelated and did not change the F1-94 result.
