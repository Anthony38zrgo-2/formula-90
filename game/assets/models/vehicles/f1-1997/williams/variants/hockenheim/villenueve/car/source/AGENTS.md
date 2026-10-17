# Fuente editable del coche

Fuente vigente ya presente: williams_fw19_hockenheim_1997_car.blend. Copiada sin alterar desde el depósito y reabierta: 64 objetos, 16 imágenes usadas empaquetadas. Procedencia en ../../reports/authoring_source_promotion.json. Este estado sustituye la palabra futuro del párrafo siguiente; normalización de nombres/exportación todavía pendientes.

Destino futuro williams_fw19_hockenheim_1997_car.blend, copiado y verificado desde season/1997/williams_completed/williams_fw19_textured.blend. Conservar fuente completa con las cuatro ruedas; GLB no reemplaza blend. .gdignore excluye esta autoría de Godot. Normalizar nombres con mapa original/final y consumidores; GEO_STEERING_COLUMN_SHAFT pendiente. Preservar geometría/UV y densidad; matrices de exportación solo en copias. Texturas propias en textures, además empaquetadas. Cada edición requiere backup, hash y reapertura.

## Reglas comunes

- Leer el AGENTS padre y D:/Formula90s/instrucciones.md; pipeline plan sprint -> backlog -> implementación -> revisión -> aprobación humana -> cierre.
- Antes de cambiar: verificar rama, HEAD, git status y respaldar fuentes afectadas. No alterar ramas sucias ni cambios ajenos; no git add -A. Esta estructura es documental y los assets/escenas indicados siguen pendientes.
- Intermedios en scratch, promoción de archivos concretos con tools/common/output_policy.py. Conservar ambos blends y procedencia; no convertir la promoción local en permiso de redistribución pública de medios de terceros.
- Carpetas nuevas incluyen su AGENTS por petición del usuario, incluso hojas de medios en este programa. No crear archivos vacíos para simular entregables.
- Godot mediante Fulviuus/godot-mcp; sesiones de juego solo por scripts/run_f1_94.ps1 tras soportar Williams explícitamente y mantener BUILD/HEAD. No usar el launcher actual como si ya aceptara esta variante.
- Las limitaciones, pruebas pendientes y contratos de piloto/física se reportan explícitamente. Fuentes preparadas no equivalen a coche probado en Fuji.
