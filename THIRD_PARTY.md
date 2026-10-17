# Dependencias de terceros

Este archivo registra tanto dependencias incorporadas como dependencias evaluadas y rechazadas. Una evaluación no implica que su código forme parte del juego.

## Créditos de la comunidad GP4 — F1 1997 Season Mod v2.0

El trabajo de recuperación y adaptación visual de Formula90s utiliza como fuente local el **F1 1997 Season Mod v2.0**, organizado por **MSchumi24**. Agradecemos a sus creadores, conversores y colaboradores. La adaptación a Blender/GLB no reemplaza la autoría de los modelos, texturas y demás recursos originales.

Las atribuciones siguientes proceden de `Data/Files/Documentation/credits.txt` y `help.txt` incluidos en el carset. En esta instalación se conservan en `game/assets/season/1997/extracted/combined/`. Se distinguen los créditos del conjunto del mod de los recursos efectivamente adaptados: el candidato visual Williams utiliza la carrocería Williams y los recursos de ruedas/neumáticos; esta lista no implica que todos los componentes del mod estén integrados en el juego.

### Modelos de coches y actualizaciones

| Equipo | Créditos indicados por el mod |
|---|---|
| Arrows | Kefla, MSchumi24 y Hega; actualizaciones de SkaningeN |
| Williams | VRC, MSchumi24 y Hega; actualizaciones de SkaningeN |
| Ferrari | VRC, MSchumi24 y Hega; actualizaciones de SkaningeN |
| Benetton | Conversión desde el mod F1C 1997 por Intweb; actualizaciones de quickslick y SkaningeN |
| McLaren | Soulbringer; actualizaciones de SkaningeN |
| Jordan | Hega |
| Prost | Conversión desde el mod F1C 1997 por Intweb; actualizaciones de quickslick y SkaningeN |
| Sauber | Conversión desde el mod F1C 1997 por Intweb; actualizaciones de quickslick y SkaningeN |
| Tyrrell | Conversión desde el mod F1C 1997 por Intweb; actualizaciones de quickslick y SkaningeN |
| Minardi | Conversión desde el mod F1C 1997 por Intweb; actualizaciones de quickslick y SkaningeN |
| Stewart | Conversión desde el mod F1C 1997 por Intweb; actualizaciones de quickslick y SkaningeN |
| Lola | Conversión desde el mod F1C 1997; actualización de Turbo Lover; el archivo de créditos no identifica al conversor |

### Otros recursos del carset

| Recurso | Créditos indicados por el mod |
|---|---|
| Cockpit y sus texturas | Enzojz; texturas de MSchumi24 |
| Modelo de casco Arai | jvinu2000 |
| Texturas de cascos | jvinu2000 —Ferrari y Williams—, BlackToof, Hega, SkaningeN, DaiMon, KerleyF1 y MailsonTeles —Larini— |
| Personal de boxes y accesorios de boxes | MSchumi24 |
| Comisarios | Fongu |
| Paneles de boxes | Tony (b-tone) |
| Llantas y neumáticos | Ulf (Alesi_fan) y Turbo Lover |
| Volantes | Hega; volante interior genérico de Genison Kobe |
| Viseras | MSchumi24 |
| HUD 1994 | Duffer |
| Defpanel 2009 | Tony (b-tone) y Ruboy |
| Teamart | Tony (b-tone) y MSchumi24 |
| Pantallas de carga | SkaningeN y MSchumi24 |
| Física y rendimiento | Gitanes |
| Sonidos | Mitsuro Sano |
| Sonidos de eventos | carl_gpgames |
| Bruma atmosférica | Isaint |
| GPaedia | petrus |
| Fondos de menú | MSchumi24 |
| Música de fondo | Procedente de Formula One 97, según los créditos del carset |
| Parche de límite de memoria | Lee71 |
| Configuración CSM | MSchumi24, petrus, Flash Formula y otros colaboradores no individualizados en los créditos |
| Trackpack | plodekk y cleberpister |

### Herramientas y agradecimientos

- **ZaZ**: CSM, TSM y las herramientas reconocidas por los créditos originales del mod.
- **Diego Noriega (Öggo)**: GP4 Mesh Tools para Blender, utilizado para interpretar e importar los modelos GP4. Atribución verificada en el `gp4form.txt` del bundle local V2.3.

Los derechos de los recursos permanecen con sus autores originales, conforme a la documentación del carset. Estos créditos no conceden permisos adicionales de redistribución. Los paquetes, extracciones y candidatos se mantienen como contenido local privado; su organización y la adaptación de texturas se documentan en [Texturas y organización de GP4 1997](game/assets/season/1997/TEXTURAS_Y_ORGANIZACION.md).

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
