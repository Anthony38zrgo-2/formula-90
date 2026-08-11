# SQLite CLI local (project-local tooling)

Binarios SQLite de uso local para scripts de diagnóstico de Formula90.
Se instalan dentro del repo para no depender de SQLite en el PATH del sistema.

## Instalación

```powershell
.\.agents\scripts\install-sqlite.ps1
```

El script es idempotente: descarga desde sqlite.org solo si `sqlite3.exe`
local no funciona, extrae en TEMP, copia los binarios aquí y verifica.

## Resolución en scripts

`Get-SqliteExe` (en `.agents/scripts/_common.ps1`) usa este orden:

1. `.agents/tools/sqlite/sqlite3.exe` (proyecto local)
2. `sqlite3` desde PATH (fallback)
3. no disponible

Los scripts nunca modifican PATH de máquina ni de usuario.

## Contenido

| Archivo               | Uso                          |
|-----------------------|------------------------------|
| `sqlite3.exe`         | CLI principal                |
| `sqldiff.exe`         | Diff entre bases SQLite      |
| `sqlite3_analyzer.exe`| Análisis de uso/estadísticas |
| `sqlite3_rsync.exe`   | Sincronización de tablas     |
| `version.json`        | Manifiesto de versión        |

Los binarios no se versionan en Git (ver `.agents/.gitignore`); `version.json`
y este README sí, para reproducibilidad.
