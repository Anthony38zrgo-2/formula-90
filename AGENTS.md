# Formula-90 Agent Protocol

PIPELINE:
sprint planning -> backlog item -> implement -> review -> human gate -> sprint retrospective -> done

REBUILD SAFETY: nunca mezclar cambios ajenos ni crear commits WIP masivos; al recompilar todo, no reutilizar DLL, objetos o cachés ignorados de otra rama: verificar `HEAD`, limpiar Cargo/SCons/`game/.godot` y confirmar el `BUILD` resultante.

PROVENANCE: antes de modificar, registrar y verificar rama, HEAD y git status; no cambiar/resetear una rama sucia sin inventario y respaldo aprobado.

COMMIT SCOPE: commits atómicos con rutas explícitas; prohibidos git add -A y mezclar cambios previos, generados o ajenos sin revisión de git diff --cached.

RUNTIME PARITY: el runtime debe rechazar binarios cuyo BUILD no coincida con HEAD; cada script de ejecución debe compilar o validar esa correspondencia antes de iniciar Godot.