# Texturas del piloto

Conservar atlas originales realmente usados y los generados jacques_villeneuve_williams_1997_sleeves_base_color.png y jacques_villeneuve_williams_1997_gloves_base_color.png. Azul del torso como referencia, líneas blancas y negro, sin logos/números en nuevos brazos/guantes. Casco y torso conservan sus livery originales. Conectar mapas nuevos directo a Base Color y empaquetar; no activar materiales históricos con Ferrari/44. Procedencia image_gen, prompts y hashes en scratch/williams_sleeve_image_generation, trasladar metadata compacta a reports al publicar. No borrar atribuciones originales. AGENTS de hoja solicitado por usuario.

## Reglas comunes

- Leer el AGENTS padre y D:/Formula90s/instrucciones.md; pipeline plan sprint -> backlog -> implementación -> revisión -> aprobación humana -> cierre.
- Antes de cambiar: verificar rama, HEAD, git status y respaldar fuentes afectadas. No alterar ramas sucias ni cambios ajenos; no git add -A. Esta estructura es documental y los assets/escenas indicados siguen pendientes.
- Intermedios en scratch, promoción de archivos concretos con tools/common/output_policy.py. Conservar ambos blends y procedencia; no convertir la promoción local en permiso de redistribución pública de medios de terceros.
- Carpetas nuevas incluyen su AGENTS por petición del usuario, incluso hojas de medios en este programa. No crear archivos vacíos para simular entregables.
- Godot mediante Fulviuus/godot-mcp; sesiones de juego solo por scripts/run_f1_94.ps1 tras soportar Williams explícitamente y mantener BUILD/HEAD. No usar el launcher actual como si ya aceptara esta variante.
- Las limitaciones, pruebas pendientes y contratos de piloto/física se reportan explícitamente. Fuentes preparadas no equivalen a coche probado en Fuji.
