# Dependencias de terceros

Este archivo registra tanto dependencias incorporadas como dependencias evaluadas y rechazadas. Una evaluación no implica que su código forme parte del juego.

## stb

- Repositorio: <https://github.com/nothings/stb>
- Archivos incorporados: `third_party/stb/stb_image.h`, `stb_image_write.h` y `LICENSE`.
- Licencia: dominio público o MIT, a elección, según el texto incluido.
- Función: lectura y escritura de imágenes en las herramientas offline C++.
- Modificaciones locales: ninguna documentada.

## VitaVehicle — evaluado, no incorporado

- Repositorio oficial inspeccionado: <https://github.com/jreo03/g-rcp2>
- Revisión: rama `beta`, commit `d60c8d90061b14a55e395132109bb16248e191c4` (2022-11-24).
- Licencia: MIT, copyright 2022 Jreo.
- Carpeta de integración: ninguna.
- Archivos incorporados: ninguno.
- Función evaluada: dinámica raycast, suspensión, motor, transmisión y slip.
- Dependencias/estructura: proyecto Godot 3 completo con autoload, scripts, escenas, UI, input y assets; escala de unidad 0.30592.
- Modificaciones locales: ninguna.
- Limitaciones: no es compatible directamente con Godot 4.7.1 ni con la política de runtime C++20. Requiere un port sustancial expresamente fuera de alcance.

## Racing Cameras — evaluado, no incorporado

- Repositorio oficial: <https://github.com/Skaruts/racing_cameras>
- Revisión: tag `v0.3`, commit `eb2af2d3a7a07e8e09169bc093aba8942fbb5c4c` (el manifiesto interno declara 0.2).
- Licencia: MIT, copyright 2024 Skaruts.
- Carpeta de integración: ninguna.
- Archivos incorporados: ninguno.
- Función evaluada: cámaras mounted, chase, orbit, cockpit y track, más autoload administrador.
- Dependencias: GDScript Godot 4.2; el chase espera `PhysicsBody3D.linear_velocity` y opcionalmente `get_steering_input()`.
- Modificaciones locales: ninguna.
- Limitaciones: runtime GDScript no permitido por este proyecto; seguimiento por dirección de velocidad e interpolación dependiente del frame incompatibles con los criterios de Formula-90s. Adaptarlo exigiría un fork material.

## DirectionalSprite3D — evaluación bloqueada, no incorporado

- Fuente pública enlazada: <https://github.com/MeagherGames/addons/tree/main/addons/DirectionalSprite3D/addons/directional_sprite_3d>
- Revisión: no disponible; el repositorio devuelve `Repository not found` al 2026-08-05.
- Licencia: no verificable.
- Carpeta de integración: ninguna.
- Archivos incorporados: ninguno.
- Función evaluada: selección genérica de sprites direccionales.
- Modificaciones locales: ninguna.
- Limitaciones: sin repositorio accesible, commit ni licencia no es legal ni reproducible distribuirlo. No se infiere su API desde referencias de terceros.

## xBRZ (3xBRZ upscaler) — gate de licencia, bloqueado, no incorporado

- Fuente pública oficial: <https://sourceforge.net/projects/xbrz/> (proyecto "xBRZ: Scale by rules" por Zenju).
- Revisión: página del proyecto y metadatos SourceForge verificados el 2026-08-10; última actualización 2026-02-23.
- Licencia: **GNU GPL v3.0** (declarada en SourceForge). El repositorio Formula-90s es **MIT**; una implementación derivada de xBRZ no es redistribuible bajo MIT sin compatibilización de licencias.
- Carpeta de integración: ninguna.
- Archivos incorporados: ninguno.
- Función evaluada: upscaling offline 3x de tarjetas de vegetación (tree cards) en Texture Forge.
- Modificaciones locales: ninguna.
- Conclusión de compatibilidad: **no compatible**. No hay implementación MIT aprobada verificable ni ruta clean-room documentada aprobada dentro de este incremento, por lo que el trabajo 3xBRZ queda bloqueado en el gate de licencia (instrucciones sección 47). El upscaler offline se mantiene como trabajo futuro condicionado a una implementación con licencia compatible o derivación no-derivativa autorizada.
