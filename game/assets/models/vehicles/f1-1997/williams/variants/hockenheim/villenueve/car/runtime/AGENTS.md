# Exportaciones del coche

Cinco GLB: williams_fw19_hockenheim_1997_chassis.glb y cuatro wheel_front_left/front_right/rear_left/rear_right con el mismo prefijo. Chasis excluye ruedas/piloto; cada rueda usa origen propio y offset registrado. +X derecha, +Y arriba, -Z morro, metros. Preservar SpinVisual/BrakeStatic y eje X local sin doble autoridad. No .gdignore aquí. Exportar desde fuentes normalizadas en scratch, reimportar, medir, validar materiales/imágenes y promover por output_policy. No editar GLB a mano ni copiar GLB histórico. AGENTS aquí es excepción de hoja solicitada por usuario.

## Reglas comunes

- Leer el AGENTS padre y D:/Formula90s/instrucciones.md; pipeline plan sprint -> backlog -> implementación -> revisión -> aprobación humana -> cierre.
- Antes de cambiar: verificar rama, HEAD, git status y respaldar fuentes afectadas. No alterar ramas sucias ni cambios ajenos; no git add -A. Esta estructura es documental y los assets/escenas indicados siguen pendientes.
- Intermedios en scratch, promoción de archivos concretos con tools/common/output_policy.py. Conservar ambos blends y procedencia; no convertir la promoción local en permiso de redistribución pública de medios de terceros.
- Carpetas nuevas incluyen su AGENTS por petición del usuario, incluso hojas de medios en este programa. No crear archivos vacíos para simular entregables.
- Godot mediante Fulviuus/godot-mcp; sesiones de juego solo por scripts/run_f1_94.ps1 tras soportar Williams explícitamente y mantener BUILD/HEAD. No usar el launcher actual como si ya aceptara esta variante.
- Las limitaciones, pruebas pendientes y contratos de piloto/física se reportan explícitamente. Fuentes preparadas no equivalen a coche probado en Fuji.
