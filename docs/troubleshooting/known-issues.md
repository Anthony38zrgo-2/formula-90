# Known Issues

Registro de problemas actuales o patrones de error activos en el proyecto. 

*Instrucciones de uso: Consulta este documento en DIAGNOSTIC MODE antes de intentar solucionar problemas recurrentes.*

---

## 1. Oscilación de Cámara en Alta Velocidad

**Symptom:**
La cámara (`ArcadeChaseCamera` en C++ o equivalentes) tiembla violentamente cuando el vehículo toma curvas prolongadas a muy alta velocidad.

**Likely causes:**
Conflictos en el orden de procesamiento físico (Interpolación de RigidBody3D de GEVP chocando con el `_process` de la cámara) o estabilización redundante en múltiples ejes.

**Checks:**
- Revisar si `test_field.tscn` tiene activada interpolación asíncrona.
- Verificar el script de GEVP que pasa los transforms.

**Do not:**
Añadir más capas de `lerp()` matemático sin antes identificar qué sistema es dueño de la rotación absoluta del frame actual.

**Related commits:**
- `8c39a10`, `2da7f5f`, `9642eed`, `b8901c3`

---

## 2. Los árboles no se ven más grandes aunque se duplique el GLB

**Symptom:**
Se escala la geometría GLB de vegetación (o el asset) y las copas finales en la pista no cambian, o solo cambian parcialmente.

**Likely causes:**
El compilador semántico deriva la escala final desde el contrato de altura objetivo:
`scale = target_height_m / asset_height_m`. Duplicar solo la geometría del asset
se cancela en la compilación; la escala visible la gobierna
`target_height_m` de la zona en el catalog + `tree_visual_scale` en
`layout_config.json`.

**Checks:**
- Cambiar `tree_visual_scale` en `blender/track_pipeline/layouts/la_chutana/layout_config.json` (no un literal en código genérico).
- Recompilar y revisar `compiled_layout.json`: `vegetation[].target_height_m` y `footprint_radius_m`.
- Verificar determinismo compilando dos veces (hash idéntico).

**Do not:**
Escalar el GLB o la textura para cambiar el tamaño en el mundo; el contrato
semántico es el dueño del scale.

## 3. Halo oscuro/coloreado en tarjetas de vegetación transparentes

**Symptom:**
Las tarjetas de árboles muestran un borde oscuro o azul/verde alrededor de la copa al verse en el mundo.

**Likely causes:**
RGB contaminado en píxeles totalmente transparentes o procesado de alpha sin
premultiplicación. Los filtros/upscalers de tarjetas transparentes deben ser
premultiplicado-safe y limpiar RGB en alpha=0 para evitar halos.

**Checks:**
- Inspeccionar RGB de píxeles con alpha=0 en la tarjeta fuente.
- Verificar bottom anchor y alpha coverage tras el pipeline v2
  (`prepare_raw_vegetation_v2.py` / `validate_raw_vegetation_v2.py`).
- Si se introduce un upscaler de tarjetas, validar bordes transparentes sin
  fringes (gate de licencia obligatorio: repo MIT, xBRZ GPLv3 bloqueado).

**Do not:**
Aplicar filtros por-frame en Godot a tarjetas de vegetación; es un problema de
build/import, no de runtime.
