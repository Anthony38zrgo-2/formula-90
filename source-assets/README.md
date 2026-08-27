# Source assets

Fuentes editables y material de procedencia usado por las herramientas de
Formula90s.

Esta zona puede contener, cuando un migration slice los incorpore:

- archivos Blender, SVG y heightmaps;
- WAV originales;
- imágenes y texturas fuente;
- modelos de alta resolución;
- referencias, licencias y evidencia de procedencia.

Reglas:

- `game/` no lee directamente desde este directorio;
- los subdirectorios se crean solo al migrar contenido real;
- los outputs generados pertenecen a `scratch/` durante preview o a
  `game/assets`/`game/sounds` después de promoción;
- incorporar archivos grandes a Git LFS requiere un backlog item explícito.

Mapa transitorio: `docs/architecture-migration-map.md`.
