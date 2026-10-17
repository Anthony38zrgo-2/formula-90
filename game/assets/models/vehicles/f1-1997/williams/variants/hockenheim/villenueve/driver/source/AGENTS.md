# Fuente editable del piloto

Fuente vigente ya presente: jacques_villeneuve_williams_fw19_1997_driver.blend. Copiada sin alterar desde el depósito y reabierta: ocho objetos, una armadura, ocho imágenes usadas empaquetadas incluidos los atlas generados. Procedencia en ../../reports/authoring_source_promotion.json. No es un GLB publicado ni un rig completo validado.

Destino jacques_villeneuve_williams_fw19_1997_driver.blend, copia verificada del candidato con ocho objetos y atlas generados. .gdignore evita importación automática. Conservar casco, visor, torso/piernas/botas/arnés, mangas, guantes, controles y armadura. No renombrar huesos sin migrar consumidores/pesos. Raíz incorpora giro 180 grados; no duplicarlo. Hombros ajustados a zonas blancas conservando mano mundial. Solo mangas/guantes están skinned; torso/casco rígidos. Registrar bind/rest y pose inicial antes de exportar. Consultar COMPONENTES_PILOTO.md y reportes recientes, no intentos históricos fallidos.

## Reglas comunes

- Leer el AGENTS padre y D:/Formula90s/instrucciones.md; pipeline plan sprint -> backlog -> implementación -> revisión -> aprobación humana -> cierre.
- Antes de cambiar: verificar rama, HEAD, git status y respaldar fuentes afectadas. No alterar ramas sucias ni cambios ajenos; no git add -A. Esta estructura es documental y los assets/escenas indicados siguen pendientes.
- Intermedios en scratch, promoción de archivos concretos con tools/common/output_policy.py. Conservar ambos blends y procedencia; no convertir la promoción local en permiso de redistribución pública de medios de terceros.
- Carpetas nuevas incluyen su AGENTS por petición del usuario, incluso hojas de medios en este programa. No crear archivos vacíos para simular entregables.
- Godot mediante Fulviuus/godot-mcp; sesiones de juego solo por scripts/run_f1_94.ps1 tras soportar Williams explícitamente y mantener BUILD/HEAD. No usar el launcher actual como si ya aceptara esta variante.
- Las limitaciones, pruebas pendientes y contratos de piloto/física se reportan explícitamente. Fuentes preparadas no equivalen a coche probado en Fuji.
