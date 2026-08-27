# Architecture inventory

ARCH-006 mantiene un inventario estático y reproducible de deuda en fronteras
arquitectónicas.

## Generar

```powershell
python tools/common/architecture_inventory.py `
  --repo . `
  --output reports/architecture/architecture-inventory.json
```

El JSON:

- no contiene timestamp ni rutas absolutas;
- ordena findings de forma determinista;
- excluye caches, binarios, reports, scratch y dependencias vendorizadas;
- puede regenerarse para comparar el avance de cada migration slice.

## Categorías

| Categoría | Significado |
|---|---|
| `cross_zone_literal` | Literal hacia una zona legacy o runtime |
| `parent_arithmetic` | Resolución de raíz mediante `parents[N]` |
| `runtime_output_default` | Default/config que apunta a contenido runtime |
| `runtime_write_candidate` | Archivo que combina referencia runtime y operación de escritura |
| `game_source_dependency` | Código/config bajo `game/` que referencia fuentes legacy |

El inventario es heurístico y **no bloquea FAST ni FULL**. Un finding es un
candidato para inspección, no una prueba automática de defecto. Su baseline no
se reduce borrando entradas manualmente: se cambia código y se regenera.

## Uso durante migración

Cada slice consulta solamente sus rutas relevantes. Al cerrar:

1. regenera el inventario;
2. compara counts y findings del dominio;
3. justifica cualquier finding nuevo;
4. no exige resolver deuda de otros dominios.

La futura automatización puede bloquear únicamente regresiones nuevas, nunca la
baseline histórica completa.
