# Estándar de Iluminación y Entorno de Escena (Scene Environment Standard)

## Propósito y Autoridad

Este documento define el **estándar canónico obligatorio** para la configuración de escenas 3D en **Formula-90**. Establece los parámetros visuales para garantizar una estética arcade auténtica de los años 90 (estilo *Sega Rally*, *Daytona USA*, *F1 '97*), con superficies mate no reflectivas, iluminación balanceada sin quemar blancos ni empastar sombras, y sombra dinámica exclusiva para el monoplaza.

---

## 1. Configuración del `WorldEnvironment`

```gdscript
[sub_resource type="Environment" id="Environment_Standard"]
background_mode = 2                   # ENV_BG_SKY
ambient_light_source = 2              # AMBIENT_SOURCE_COLOR (Relleno constante)
ambient_light_color = Color(0.68, 0.66, 0.62, 1.0) # Tono neutro/cálido de tierra
ambient_light_energy = 0.72           # Relleno suficiente para evitar sombras negras
reflected_light_source = 0            # REFLECTION_SOURCE_DISABLED (OBLIGATORIO)
tonemap_mode = 1                      # TONE_MAPPER_REINHARD (Compresión suave)
tonemap_exposure = 0.95               # Exposición equilibrada
glow_enabled = false                  # OBLIGATORIO: Desactivado
fog_enabled = false                   # Niebla global desactivada (usar distancias si aplica)
```

### Reglas Clave:
- **`reflected_light_source = 0` (DISABLED)**: **Prohibido usar reflexión IBL del cielo**. Esto previene la neblina blanquecina brillante y el falso efecto *bloom* sobre el asfalto y el desierto.
- **`tonemap_mode = 1` (Reinhard)**: Evita la curva en "S" agresiva de *Filmic* que empasta oscuros y sobre-satura contrastes.
- **`glow_enabled = false`**: En ningún caso debe activarse post-procesado de *glow* o *bloom*.

---

## 2. Configuración de la Luz Solar (`DirectionalLight3D`)

```gdscript
[node name="DirectionalLight3D" type="DirectionalLight3D" parent="."]
light_color = Color(1.0, 0.98, 0.94, 1.0) # Blanco cálido luz día (255, 250, 240)
light_energy = 0.80                      # Intensidad equilibrada con la luz ambiental
light_specular = 0.0                    # OBLIGATORIO: 0.0 (Elimina brillo plástico)
shadow_enabled = true                   # Sombra activa para el vehículo
shadow_opacity = 0.55                   # Sombra de contacto traslúcida natural
shadow_blur = 1.5                       # Suavizado de bordes anti-aliasing
shadow_bias = 0.02                      # Sesgo para evitar shadow acne
directional_shadow_max_distance = 40.0  # Rango enfocado al radio de la cámara/coche
directional_shadow_blend_splits = true  # Transición suave entre cascadas
```

### Reglas Clave:
- **`light_specular = 0.0`**: Las superficies no deben tener lóbulo especular Cook-Torrance del sol, preservando el acabado mate clásico.
- **`shadow_opacity = 0.55`**: La sombra del coche nunca debe ser negro puro `#000000`; debe integrarse de forma traslúcida con la textura del asfalto.
- **`directional_shadow_max_distance = 40.0`**: Concentra toda la resolución del mapa de sombras en el monoplaza.

---

## 3. Presupuesto de Sombras del Escenario (`GeometryInstance3D`)

Para replicar el rendimiento y claridad de los 90s, el escenario **no proyecta sombras dinámicas en cascada**:

| Elemento de Escena | `cast_shadow` | `gi_mode` | Razón |
| :--- | :--- | :--- | :--- |
| **Monoplaza / Vehículo** | **`1` (ON)** | `0` (Disabled) | Único emisor de sombra dinámica proyectada. |
| **Pista y Asfalto (`GeneratedTrack`)** | **`0` (OFF)** | `0` (Disabled) | Recibe luz difusa y sombra del coche; no proyecta. |
| **Muros y Barreras (`TireWall / Jersey / Armco`)** | **`0` (OFF)** | `0` (Disabled) | Evita sombras duras laterales sobre el trazado. |
| **Vegetación (`Trees / Bushes / Grass`)** | **`0` (OFF)** | `0` (Disabled) | Elimina ruido visual de sombras de billboards. |
| **Props y Señalética (`Signs / Flags / People`)** | **`0` (OFF)** | `0` (Disabled) | Renderizado rápido como cards 2D / props. |
| **Montañas 3D y Cielo (`BackgroundMountains3D`)** | **`0` (OFF)** | `0` (Disabled) | Fondos lejanos no proyectan sombras. |

---

## 4. Estándar de Materiales PBR del Circuito

Todos los materiales deben mantener valores altos de rugosidad para conservar la estética mate:

| Material | Rugosidad (`Roughness`) | Metálico (`Metallic`) | Alpha Mode |
| :--- | :--- | :--- | :--- |
| **Asfalto (`F90_Asphalt`)** | `0.95` | `0.0` | Opaco |
| **Terreno / Desierto (`F90_Ground`)** | `1.00` | `0.0` | Opaco |
| **Borde / Shoulder (`F90_Shoulder`)** | `1.00` | `0.0` | Opaco |
| **Pianos Rojos / Blancos (`F90_Curb`)** | `0.80 - 0.84` | `0.0` | Opaco |
| **Líneas de Pista (`F90_EdgeLine`)** | `0.86` | `0.0` | Opaco |
| **Muros Neumáticos / Concreto** | `0.98 - 1.00` | `0.0` | Opaco |
| **Guardrail Metálico Armco** | `0.60` | `0.70` | Opaco |
| **Vegetación 3D (Árboles / Arbustos)** | `0.98` | `0.0` | Alpha Clip / Dithered |
| **Hierba / Césped Card Visual** | `1.00` | `0.0` | Alpha Clip / Dithered |
