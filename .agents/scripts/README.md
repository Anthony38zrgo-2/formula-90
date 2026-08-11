# `.agents/scripts` — Operations bundle

Scripts de administración y diagnóstico del POC Rust + SQLite.

Estos scripts NO sustituyen al runtime Rust. `agentdb` sigue siendo la interfaz
normal de los agentes. Los scripts PowerShell son herramientas humanas para
comprobar que la base está inicializada, inspeccionar tablas y diagnosticar el POC.

## Estado esperado de las tablas

### Seed / conocimiento compilado

Estas tablas se reconstruyen a partir de los JSON con `agentdb seed`:

- `instructions`
- `knowledge_entries`
- `knowledge_terms`
- `common_problems`
- `problem_terms`
- `sources`
- `skill_registry`
- `agent_registry`

No tienen por qué crecer durante cada tarea. Cambian cuando se modifica el corpus
JSON y se vuelve a ejecutar el seed.

### Runtime

- `context_cache`

Solo crecerá si el runtime implementa y utiliza operaciones de escritura de
contexto. Una tabla vacía no implica por sí sola que el POC esté roto.

## Comandos principales

La ubicación obligatoria es:

```text
<repo>\.agents\scripts
```

Ejemplo:

```text
D:\Formula90s\.agents\scripts
```

Primero valida que todos los scripts puedan parsearse:

```powershell
.\00-preflight.ps1
```

Después:

Desde la raíz del repositorio:

```powershell
.\.agents\scripts\00-env.ps1
.\.agents\scripts\01-health.ps1
.\.agents\scripts\02-db-schema.ps1
.\.agents\scripts\03-db-counts.ps1
```

Ver una tabla:

```powershell
.\.agents\scripts\04-table.ps1 knowledge_entries
.\.agents\scripts\04-table.ps1 common_problems -Limit 100
.\.agents\scripts\04-table.ps1 skill_registry -Json
```

SQL arbitrario de solo lectura:

```powershell
.\.agents\scripts\05-sql-readonly.ps1 "SELECT * FROM knowledge_entries WHERE domain='godot' LIMIT 10;"
```

Consulta mediante el runtime Rust:

```powershell
.\.agents\scripts\06-agentdb-query.ps1 -Type knowledge -Domain godot -Query "RigidBody3D"
.\.agents\scripts\06-agentdb-query.ps1 -Type problem -Query "node_paths"
.\.agents\scripts\06-agentdb-query.ps1 -Type agent -Query "developer-godot"
```

Prueba funcional:

```powershell
.\.agents\scripts\08-smoke-test.ps1
```

Monitor de crecimiento:

```powershell
.\.agents\scripts\09-watch-counts.ps1 -Seconds 5
```

Reconstrucción completa:

```powershell
.\.agents\scripts\07-rebuild-db.ps1 -ConfirmRebuild
```

Antes de borrar `agents.db`, el script crea un backup timestamped en `.agents/data/`.

## Criterio mínimo de "operativo"

El POC puede considerarse operativo cuando:

1. `00-env.ps1` encuentra Rust/Cargo, SQLite y `agentdb`.
2. `01-health.ps1` termina con `HEALTHCHECK: PASS`.
3. `PRAGMA integrity_check` devuelve `ok`.
4. Las tablas seed contienen filas.
5. `agentdb validate` pasa.
6. `08-smoke-test.ps1` devuelve resultados para varios dominios.
7. Se puede borrar/reconstruir `agents.db` desde los JSON mediante `07-rebuild-db.ps1`.

La persistencia runtime es un incremento separado. Que `context_cache` esté vacío
solo significa que todavía no se han producido escrituras runtime.
