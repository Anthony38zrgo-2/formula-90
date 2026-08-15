# Prompt maestro para Codex — Refactor del background de Formula-90 a sistema por layers/parallax

Trabaja sobre el repositorio de **Formula-90** y rehace el sistema con el que Godot prepara y renderiza los backgrounds de los circuitos.

## 0. Primero: inspección obligatoria

Antes de modificar código:

1. Lee y respeta `.agents`, `AGENTS.md` y cualquier skill/instrucción del repositorio.
2. Localiza todos los archivos, escenas, scripts, shaders, recursos y configuraciones que actualmente intervienen en:
   - background del circuito,
   - cielo,
   - horizonte,
   - montañas/decorado lejano,
   - cámara,
   - carga del mapa/circuito,
   - configuración visual por circuito.
3. Determina cómo está acoplado actualmente el background al circuito y a la cámara.
4. No implementes todavía hasta producir un plan breve con:
   - archivos afectados,
   - componentes nuevos,
   - componentes a retirar o deprecar,
   - estrategia de migración sin romper circuitos existentes.

Después del análisis, procede con la implementación.

---

# 1. Referencias visuales obligatorias

Dentro de este paquete existen estas imágenes:

- `reference/00_original_reference.jpg`
  - Captura original.
  - Es la referencia principal de composición.
  - Debe estudiarse la separación aparente del fondo en planos.

- `reference/01_mountains_near.png`
  - Layer de montañas cercanas.
  - Debe colocarse delante de las montañas lejanas.
  - Debe moverse con un factor de parallax mayor.

- `reference/02_mountains_far.png`
  - Layer de montañas lejanas.
  - Debe tener menor movimiento relativo que las montañas cercanas.

- `reference/03_sky.png`
  - Layer de cielo/nubes.
  - Debe ser el plano más lejano y prácticamente estable respecto a la cámara.

No trates estas imágenes como una sola textura plana. El objetivo es reproducir explícitamente el principio visual de la captura original mediante **layers independientes**.

---

# 2. Objetivo arquitectónico

Crear un sistema reusable de background para circuitos basado en composición de capas.

Conceptualmente:

```text
Circuit
 └── BackgroundController
      ├── SkyLayer
      ├── FarMountainsLayer
      ├── NearMountainsLayer
      └── OptionalForegroundLayers...
```

El background NO debe estar hardcodeado dentro del mapa ni depender de una escena específica de un circuito.

Debe ser un sistema desacoplado que pueda cargarse con una configuración externa.

El circuito solo debe declarar qué preset/background utiliza.

---

# 3. Requisitos funcionales

## 3.1 Layers mínimos

Implementar como mínimo:

```text
Layer 0 = sky
Layer 1 = far_mountains
Layer 2 = near_mountains
```

Debe ser posible añadir después:

```text
Layer 3 = distant_buildings
Layer 4 = trees
Layer 5 = grandstands
Layer 6 = foreground_details
```

sin tener que modificar la lógica base.

Cada layer debe ser una unidad independiente.

## 3.2 Configuración externa

La composición debe definirse mediante un JSON o Resource externo por circuito/preset.

Ejemplo conceptual:

```json
{
  "id": "snes_mountain_day",
  "layers": [
    {
      "id": "sky",
      "texture": "res://assets/backgrounds/snes_mountain_day/sky.png",
      "depth": 0,
      "parallax_x": 0.02,
      "parallax_y": 0.00,
      "scale": 1.0,
      "offset_x": 0.0,
      "offset_y": 0.0,
      "repeat_x": true,
      "repeat_y": false,
      "pixel_snap": true
    },
    {
      "id": "far_mountains",
      "texture": "res://assets/backgrounds/snes_mountain_day/mountains_far.png",
      "depth": 1,
      "parallax_x": 0.08,
      "parallax_y": 0.01,
      "scale": 1.0,
      "offset_x": 0.0,
      "offset_y": 0.0,
      "repeat_x": true,
      "repeat_y": false,
      "pixel_snap": true
    },
    {
      "id": "near_mountains",
      "texture": "res://assets/backgrounds/snes_mountain_day/mountains_near.png",
      "depth": 2,
      "parallax_x": 0.18,
      "parallax_y": 0.02,
      "scale": 1.0,
      "offset_x": 0.0,
      "offset_y": 0.0,
      "repeat_x": true,
      "repeat_y": false,
      "pixel_snap": true
    }
  ]
}
```

Los valores son iniciales y deben poder ajustarse sin recompilar.

---

# 4. Render esperado

El sistema debe recrear la sensación de los juegos de carreras SNES/16-bit:

```text
SKY
  movimiento casi nulo

FAR MOUNTAINS
  movimiento lento

NEAR MOUNTAINS
  movimiento claramente mayor

TRACK / WORLD
  movimiento normal de cámara
```

La diferencia de velocidad entre layers debe crear profundidad.

No usar desenfoque para simular profundidad.

La profundidad debe venir principalmente de:
- velocidad relativa,
- escala,
- posición vertical,
- orden de dibujo,
- contraste/paleta propia del asset.

---

# 5. Estrategia recomendada en Godot

Evalúa primero qué solución encaja mejor con la versión actual del proyecto.

Preferencia:

- `Parallax2D` / sistema equivalente actual de Godot 4.x, o
- un `BackgroundController` propio que calcule offsets desde la cámara.

No uses una solución que dependa de la posición física 3D de las montañas salvo que exista una razón técnica fuerte.

Este background debe comportarse como una composición visual, no como geometría del circuito.

Debe seguir siendo válido aunque el circuito 3D cambie.

---

# 6. Cámara y movimiento

El `BackgroundController` debe recibir únicamente la información mínima necesaria de la cámara.

Evitar:

```text
background -> conoce circuito completo
background -> conoce vehículo
background -> modifica cámara
background -> depende de físicas
```

Preferir:

```text
Camera / CameraService
        |
        v
BackgroundController
        |
        +--> layer offset
```

El background debe reaccionar a:
- yaw / desplazamiento horizontal aparente,
- opcionalmente pitch / desplazamiento vertical,
- FOV si técnicamente afecta al encuadre.

No debe reaccionar directamente a:
- velocidad del coche,
- RPM,
- suspensión,
- físicas de ruedas.

Si se necesita velocidad para suavizado, debe derivarse de la cámara y no del vehículo.

---

# 7. Pixel art: requisitos obligatorios

Estas referencias usan pixel art.

Para cada textura de background:

- desactivar filtrado bilinear,
- usar nearest-neighbor,
- evitar mipmaps si deterioran el pixel art,
- evitar compresión destructiva,
- evitar suavizado automático,
- conservar alpha,
- usar pixel snapping cuando corresponda.

No escalar de forma que produzca shimmering innecesario.

Si el proyecto renderiza internamente a baja resolución, integrar el background con ese pipeline.

El sistema debe soportar que los PNG fuente tengan resolución superior pero sigan siendo presentados con acabado pixelado.

---

# 8. Repetición horizontal

Los layers deben soportar repetición horizontal.

Requisito:

```text
camera rotates/moves
        |
        v
layer offset changes
        |
        v
texture wraps seamlessly
```

Si las imágenes actuales no son perfectamente tileables:

1. NO deformarlas automáticamente.
2. Implementar primero soporte técnico para wrapping.
3. Documentar que el arte puede requerir una versión seam-safe.
4. Permitir `repeat_x: false` para assets no tileables.

No introducir costuras visibles artificiales mediante stretching.

---

# 9. Separación circuito/background

El objetivo principal de arquitectura es que cambiar de circuito no implique tocar el sistema de background.

Ejemplo deseado:

```text
track_config.json
{
  "track_id": "test_mountain",
  "background_preset": "snes_mountain_day"
}
```

y:

```text
backgrounds/
  snes_mountain_day/
    background.json
    sky.png
    mountains_far.png
    mountains_near.png
```

El mapa solicita el preset.

El `BackgroundController` se encarga del resto.

---

# 10. Estructura sugerida

Adapta los nombres a las convenciones reales del repositorio.

Una estructura válida sería:

```text
game/
  systems/
    background/
      background_controller.gd
      background_layer.gd
      background_config.gd

  assets/
    backgrounds/
      snes_mountain_day/
        background.json
        sky.png
        mountains_far.png
        mountains_near.png
```

No fuerces esta estructura si contradice la arquitectura actual; conserva SRP y convenciones existentes.

---

# 11. API esperada

La API debe mantenerse pequeña.

Ejemplo conceptual:

```gdscript
background_controller.load_preset("snes_mountain_day")
background_controller.set_camera_source(camera)
background_controller.set_enabled(true)
```

Internamente:

```text
load config
  -> validate
  -> instantiate layers
  -> assign textures
  -> configure parallax
  -> apply pixel-art import/render settings
  -> bind camera
```

No crear una API gigante.

---

# 12. Validación

Añade validaciones claras.

Errores a detectar:

- textura inexistente,
- layer sin id,
- depth duplicado si causa ambigüedad,
- escala <= 0,
- configuración inválida,
- preset inexistente.

El sistema debe fallar de forma explícita y legible.

No debe producir silenciosamente un background negro.

---

# 13. Debug

Añade un modo debug simple que permita ver:

```text
BACKGROUND DEBUG
preset: snes_mountain_day

sky
parallax_x: 0.02
offset: ...

far_mountains
parallax_x: 0.08
offset: ...

near_mountains
parallax_x: 0.18
offset: ...
```

Debe poder activarse/desactivarse sin afectar gameplay.

También sería útil poder:
- ocultar un layer,
- congelar parallax,
- modificar temporalmente factores desde inspector/debug config.

---

# 14. Preset inicial usando las imágenes de referencia

Crea un preset inicial inspirado directamente en:

```text
reference/03_sky.png
reference/02_mountains_far.png
reference/01_mountains_near.png
```

Orden:

```text
back
|
|  sky
|  far_mountains
|  near_mountains
|
front
```

La captura:

```text
reference/00_original_reference.jpg
```

debe utilizarse para ajustar visualmente:
- horizonte,
- proporción vertical,
- distancia aparente,
- velocidad relativa,
- tamaño de montañas.

No copies HUD, pista ni coche.

Solo interesa el principio de composición del background.

---

# 15. No hacer

No:
- meter los tres layers dentro de una sola textura,
- hardcodear paths en la escena del circuito,
- duplicar lógica por cada pista,
- ligar background a físicas,
- generar montañas como meshes 3D para resolver este problema,
- usar blur como sustituto del parallax,
- usar filtrado suave,
- recompilar para cambiar velocidades de parallax,
- crear dependencias circulares entre cámara, circuito y background.

---

# 16. Compatibilidad

El refactor debe preservar los circuitos actuales.

Si hay un background legacy:

```text
legacy background
       |
       +--> adapter/fallback temporal
```

Evita una migración destructiva.

Si no existe configuración nueva para un circuito, debe existir un fallback razonable o un error claro según lo que sea más coherente con la arquitectura actual.

---

# 17. Criterios de aceptación

La tarea termina solo si:

1. El background está desacoplado del circuito.
2. Existen al menos 3 layers independientes.
3. Cada layer usa una textura distinta.
4. Los factores de parallax son configurables externamente.
5. Las imágenes pueden cambiarse sin modificar código.
6. El orden de dibujo se controla por configuración.
7. El pixel art se renderiza sin filtrado suave.
8. El sistema soporta alpha correctamente.
9. Existe soporte de repetición horizontal configurable.
10. Cambiar de preset no requiere recompilar.
11. La cámara es la única fuente de movimiento visual necesaria.
12. Hay validación de configuración.
13. Existe debug mínimo.
14. El código no rompe la carga actual de circuitos.
15. Se crea un preset funcional usando los assets de este paquete.

---

# 18. Prueba visual obligatoria

Al terminar:

1. Ejecuta el circuito de prueba disponible más simple.
2. Captura el background con el vehículo:
   - recto,
   - curva izquierda,
   - curva derecha.
3. Verifica que:
   - el cielo prácticamente no se mueve,
   - las montañas lejanas se mueven poco,
   - las montañas cercanas se mueven más,
   - no hay shimmering severo,
   - no aparecen seams inesperados,
   - no existe filtrado borroso.
4. Compara visualmente con `reference/00_original_reference.jpg`.

No busco una copia exacta de la pista del screenshot.

Busco reproducir su técnica visual de profundidad por capas.

---

# 19. Entregables

Al finalizar deja:

1. Implementación funcional.
2. Config/preset del background.
3. Assets integrados en el lugar correcto.
4. Un documento corto:
   - arquitectura final,
   - archivos modificados,
   - cómo crear un nuevo preset,
   - parámetros principales,
   - problemas conocidos.
5. Evidencia de prueba.
6. Lista de cualquier deuda técnica restante.

Haz cambios pequeños, comprobables y reversibles. Prioriza desacoplamiento, configuración externa, SRP y facilidad para iterar visualmente.
