# Procedencia y puertas de publicación

Futuros source_inventory.json, object_name_mapping.json, coordinate_and_pivot_report.json, export_validation.json, godot_import_validation.json y fuji_validation.json. Registrar fechas, rama/HEAD/dirty snapshot, hashes de fuentes y seis assets, herramientas, comandos realmente ejecutados, capturas referenciadas y defectos. No guardar estados PASS de pruebas no realizadas, ni resultados F1 2030 como Williams. Logs/capturas grandes en scratch; aquí evidencia compacta necesaria para regeneración. Mantener créditos/licencias por pieza y estado de aprobación humana.

## Reglas comunes

- Leer el AGENTS padre y D:/Formula90s/instrucciones.md; pipeline plan sprint -> backlog -> implementación -> revisión -> aprobación humana -> cierre.
- Antes de cambiar: verificar rama, HEAD, git status y respaldar fuentes afectadas. No alterar ramas sucias ni cambios ajenos; no git add -A. Esta estructura es documental y los assets/escenas indicados siguen pendientes.
- Intermedios en scratch, promoción de archivos concretos con tools/common/output_policy.py. Conservar ambos blends y procedencia; no convertir la promoción local en permiso de redistribución pública de medios de terceros.
- Carpetas nuevas incluyen su AGENTS por petición del usuario, incluso hojas de medios en este programa. No crear archivos vacíos para simular entregables.
- Godot mediante Fulviuus/godot-mcp; sesiones de juego solo por scripts/run_f1_94.ps1 tras soportar Williams explícitamente y mantener BUILD/HEAD. No usar el launcher actual como si ya aceptara esta variante.
- Las limitaciones, pruebas pendientes y contratos de piloto/física se reportan explícitamente. Fuentes preparadas no equivalen a coche probado en Fuji.
