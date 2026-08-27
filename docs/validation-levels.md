# Validation levels — FAST and FULL

Formula90s expone solamente dos entradas de validación.

## FAST

Uso durante desarrollo y migration slices:

```powershell
.\test_fast.ps1
.\test_fast.ps1 -Area Audio
.\test_fast.ps1 -Area Architecture,Vehicle
.\test_fast.ps1 -List
```

El área es explícita. FAST no intenta inferirla desde `git status`, porque un
árbol sucio puede contener trabajo ajeno y disparar validaciones no relacionadas.

Áreas iniciales:

| Área | Validación dirigida |
|---|---|
| `Architecture` | Tests de infraestructura común y output policy |
| `Audio` | Tests del tooling offline de audio |
| `Vehicle` | Validación de assets runtime F1-94 |
| `Track` | Perfil SVG y compilador de pista |
| `Runtime` | Validación canónica y paridad BUILD/HEAD sin launch interactivo |

Los slices posteriores pueden cambiar la implementación interna de un área sin
añadir niveles visibles nuevos.

## FULL

Uso al cerrar un slice, integrar un cambio transversal o preparar un merge:

```powershell
.\test_full.ps1 -Plan
.\test_full.ps1
```

FULL delega en la suite Windows existente, construye binarios, ejecuta tests y
smokes, y termina invocando `run_f1_94.ps1 -ValidateRuntimeOnly` para conservar
la paridad canónica BUILD/HEAD.

FAST no sustituye FULL al cerrar una migración. FULL tampoco debe ejecutarse
antes del human gate de un cambio perceptible solo para decidir si vale la pena.
