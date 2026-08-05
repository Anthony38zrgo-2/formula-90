# Assets de juego

`v10` es el primer vehículo y `v10_prototype` su banco original de audio. Las referencias originales permanecen bajo `references/`; aquí solo viven resultados procesados y sus metadatos. No editar manualmente archivos generados.

Importar sprites con alpha, nearest-neighbor, sin mipmaps ni compresión destructiva. Las capas continuas de motor se reproducen en loop por el mezclador C++; los cambios son one-shots. Todo asset debe tener metadatos, origen verificable y pasar `validate_assets`.
