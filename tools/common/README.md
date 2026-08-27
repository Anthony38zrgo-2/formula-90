# Common tooling infrastructure

Infraestructura estable y neutral respecto al dominio. Este directorio no debe
convertirse en un cajón de reglas de audio, vehículos, Terrain o Track.

## Output policy

`output_policy.py` implementa la frontera de escritura fail-closed:

```text
preview -> solo hijos de scratch/
promote -> solo hijos de game/assets/ o game/sounds/
```

Las raíces amplias se rechazan: un comando no puede seleccionar `scratch/`,
`game/assets/` o `game/sounds/` completos como destino.

Uso desde Python:

```python
from tools.common.output_policy import validate_output_path

decision = validate_output_path(repo_root, output, "preview")
# Escribir únicamente en decision.path después de validar.
```

Uso como CLI desde cualquier wrapper:

```powershell
python tools/common/output_policy.py `
  --repo D:\Formula90s `
  --mode preview `
  --output scratch/audio/candidate.wav
```

La política no escribe archivos. Todo nuevo entrypoint de preview o promote
debe invocarla inmediatamente antes de escribir. Los entrypoints legacy se
adoptan por dominio durante ARCH-007 y los slices AUDIO/ENV/VEH/TERR/TRACK.
