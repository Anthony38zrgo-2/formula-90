# Jacques Villeneuve — variante canónica pendiente

Estado de fuentes actualizado: car/source/williams_fw19_hockenheim_1997_car.blend y driver/source/jacques_villeneuve_williams_fw19_1997_driver.blend ya se promocionaron por petición del usuario, con copia byte a byte y reapertura en Blender. Imágenes usadas empaquetadas: coche 16, piloto 8. reports/authoring_source_promotion.json registra origen, SHA-256 y tamaño. Continuar autoría desde estas copias; el depósito conserva los originales del punto de promoción. GLB, manifest de runtime e integración siguen pendientes.

La carpeta se llama villenueve por petición explícita; identidad correcta Jacques Villeneuve, Williams FW19, 1997. No corregir el slug ni crear villeneuve en paralelo. Dueño del manifest.json de variante, procedencia, hashes de fuentes/GLB, perfil/digest, coordenadas, offsets y validaciones. Consumidores futuros: definición y escena Williams independientes, launcher canónico y sesión Fuji. Debe haber dos fuentes Blender y seis GLB; los pendientes no se fabrican como placeholders. Seguir exactamente el árbol y puertas de instrucciones.md. La creación de AGENTS no implica promoción ni integración.

## Reglas comunes

- Leer el AGENTS padre y D:/Formula90s/instrucciones.md; pipeline plan sprint -> backlog -> implementación -> revisión -> aprobación humana -> cierre.
- Antes de cambiar: verificar rama, HEAD, git status y respaldar fuentes afectadas. No alterar ramas sucias ni cambios ajenos; no git add -A. Esta estructura es documental y los assets/escenas indicados siguen pendientes.
- Intermedios en scratch, promoción de archivos concretos con tools/common/output_policy.py. Conservar ambos blends y procedencia; no convertir la promoción local en permiso de redistribución pública de medios de terceros.
- Carpetas nuevas incluyen su AGENTS por petición del usuario, incluso hojas de medios en este programa. No crear archivos vacíos para simular entregables.
- Godot mediante Fulviuus/godot-mcp; sesiones de juego solo por scripts/run_f1_94.ps1 tras soportar Williams explícitamente y mantener BUILD/HEAD. No usar el launcher actual como si ya aceptara esta variante.
- Las limitaciones, pruebas pendientes y contratos de piloto/física se reportan explícitamente. Fuentes preparadas no equivalen a coche probado en Fuji.
