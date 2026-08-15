# Manifiesto de assets generativos

## Regla de autoridad

Todo asset visual generado con IA para Formula-90 debe empezar en un input
inicial verificable y editarse a partir de él. No se acepta una generación sin
procedencia como asset final, salvo que el usuario autorice explícitamente la
creación de un nuevo baseline. Ese baseline pasa a ser un input inmutable antes
de cualquier integración.

La IA interpreta y transforma; no sustituye el ownership del asset, su licencia
ni la revisión artística.

## Estructura obligatoria

```text
<asset-family>/
├── reference/                         # inputs inmutables
├── working/                           # candidatos, croma y fuentes de edición
├── layers/ | output/                  # entregables aceptados y versionados
└── generative_asset_manifest.json     # procedencia y validación
```

No sobrescribir una versión aceptada. Publicar `*_v2`, `*_v3`, etc. El asset
referenciado por Godot se cambia únicamente después de validar la nueva versión.

## Proceso obligatorio

1. Clasificar: `EDIT`, `STYLE_TRANSFER`, `COMPOSITE`, `CUTOUT`, `PALETTE`,
   `UPSCALE` o `GENERATE_BASELINE`.
2. Inspeccionar el input y los referentes canónicos: dimensiones, alfa, paleta,
   composición y licencia/propiedad.
3. Calcular SHA-256 del input y registrarlo en el manifiesto.
4. Editar usando el input inicial como imagen de entrada explícita. Describir en
   el prompt qué composición se preserva y qué atributo cambia.
5. Guardar candidatos y cualquier fuente con croma en `working/`.
6. Aplicar sólo transformaciones deterministas posteriores: recorte, alfa,
   paleta, canvas o escalado nearest.
7. Validar hashes, rutas, dimensiones, alfa, palette, cobertura y composición.
8. Hacer revisión visual humana o con un revisor capaz de inspeccionar imágenes.
9. Promover el PNG/WEBP/texture aprobado al directorio de salida e integrar en
   Godot con su configuración de importación correspondiente.

## Manifiesto mínimo

```json
{
  "schema_version": 1,
  "asset_id": "la_chutana_background_layers",
  "asset_type": "pixel_art_parallax_layers",
  "authority": "assets-lowpoly-python/background/background_layering_codex",
  "generation": {
    "mode": "edit",
    "tool": "image_gen",
    "intent": "Transferir paleta y tratamiento canónico preservando la composición del input."
  },
  "inputs": [
    {
      "role": "edit_target",
      "path": "reference/03_sky.png",
      "sha256": "<sha256>",
      "immutable": true
    },
    {
      "role": "canonical_style_reference",
      "path": "<project-relative-path>",
      "sha256": "<sha256>",
      "immutable": true
    }
  ],
  "transformations": [
    "chroma_key_to_alpha",
    "palette_quantize",
    "nearest_resize"
  ],
  "outputs": [
    {
      "path": "layers/example_v3.png",
      "sha256": "<sha256>",
      "width": 1280,
      "height": 720,
      "alpha": "opaque|binary|graded",
      "palette_colors": 20,
      "status": "candidate|accepted"
    }
  ],
  "validation": {
    "paths_resolve": true,
    "hashes_match": true,
    "visual_review": "pending|accepted",
    "reviewer": "<person-or-image-capable-agent>"
  }
}
```

No almacenar secretos, claves API ni cadenas privadas de prompt. Registrar la
intención del prompt y las invariantes es suficiente para auditoría.

## Criterios de aceptación

Un asset está listo cuando:

- cada output tiene input, hash y transformaciones trazables;
- el input original permanece intacto;
- el asset conserva los invariantes declarados;
- el formato, dimensiones y alfa son compatibles con su consumidor;
- la configuración de Godot aplica `Nearest` para pixel art y no introduce blur;
- una revisión visual acepta el resultado;
- el manifiesto no contiene rutas absolutas ni escapes `..`.

## Prohibiciones

- Generar un asset final desde texto cuando existe un input inicial que editar.
- Tratar una descripción de estilo como sustituto del input.
- Sobrescribir el original o el asset activo sin autorización.
- Perder la fuente de croma o el input que explica una extracción de alfa.
- Declarar éxito visual sólo porque un PNG carga o porque un hash coincide.
