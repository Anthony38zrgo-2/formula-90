# Preview and promotion flow

ARCH-007 proporciona dos operaciones explícitas. No existe promoción implícita.

## Preview

```powershell
.\preview_content.ps1 `
  --source source-assets/audio/candidate.wav `
  --output scratch/audio/candidate.wav
```

Preview:

- acepta una fuente existente dentro del repositorio;
- rechaza symlinks y árboles que contengan symlinks;
- escribe únicamente bajo `scratch/`;
- usa staging para evitar outputs parciales;
- requiere `--replace` si el destino ya existe;
- permite `--plan` para validar sin escribir.

## Promote

```powershell
.\promote_content.ps1 `
  --source scratch/audio/candidate.wav `
  --output game/sounds/banks/candidate.wav
```

Promote:

- acepta únicamente fuentes bajo `scratch/`;
- escribe únicamente bajo `game/assets/` o `game/sounds/`;
- requiere `--replace` para sustituir contenido;
- conserva el contenido anterior en
  `scratch/promote-backups/<transaction>/`;
- usa staging y restaura el backup si la instalación falla;
- permite `--plan` sin modificar el filesystem.

Estas operaciones trasladan archivos; no generan contenido ni deciden si está
aprobado. El human gate ocurre antes de invocar promote. Cada dominio puede
envolver esta utilidad sin duplicar sus reglas de seguridad.

## Alcance

Los entrypoints legacy todavía no usan esta operación automáticamente. Su
adopción ocurre durante AUDIO, ENV, VEH, TERR y TRACK, evitando una reescritura
transversal antes de migrar cada dominio.
