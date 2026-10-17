# Exportación del piloto

Un GLB jacques_villeneuve_williams_fw19_1997_driver.glb, sin coche/volante/ruedas. Incluir imágenes usadas y skin válido donde existe; preservar poses y conversión común con coche. Versionar contrato híbrido si se anima, no falsificar DriverOriginalUniformGeometryContract ni reemplazar driver.glb canónico. Comprobar joint indices/pesos/matrices, reimportación y unión de hombros. Primera versión estática se etiqueta como tal. No .gdignore en runtime. AGENTS en hoja por petición explícita.

## Reglas comunes

- Leer el AGENTS padre y D:/Formula90s/instrucciones.md; pipeline plan sprint -> backlog -> implementación -> revisión -> aprobación humana -> cierre.
- Antes de cambiar: verificar rama, HEAD, git status y respaldar fuentes afectadas. No alterar ramas sucias ni cambios ajenos; no git add -A. Esta estructura es documental y los assets/escenas indicados siguen pendientes.
- Intermedios en scratch, promoción de archivos concretos con tools/common/output_policy.py. Conservar ambos blends y procedencia; no convertir la promoción local en permiso de redistribución pública de medios de terceros.
- Carpetas nuevas incluyen su AGENTS por petición del usuario, incluso hojas de medios en este programa. No crear archivos vacíos para simular entregables.
- Godot mediante Fulviuus/godot-mcp; sesiones de juego solo por scripts/run_f1_94.ps1 tras soportar Williams explícitamente y mantener BUILD/HEAD. No usar el launcher actual como si ya aceptara esta variante.
- Las limitaciones, pruebas pendientes y contratos de piloto/física se reportan explícitamente. Fuentes preparadas no equivalen a coche probado en Fuji.
