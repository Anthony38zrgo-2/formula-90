# Arquitectura General: F1 2030

## Dependencias de Motores y Lenguajes
El proyecto utiliza un modelo de procesamiento paralelo estricto:
- **GDScript (GEVP):** Gobierna el nodo físico principal del coche (`RigidBody3D`) y procesa el 100% de la dinámica de conducción.
- **C++20 (GDExtension):** Se ejecuta desde el directorio `native/` y procesa todo lo que NO es física:
  - Audio y síntesis (Motor de DSP en C++).
  - Interfaz de usuario (HUD).
  - Interacción con la cámara (`ArcadeChaseCamera`).
  - Renderización de los Sprites direccionales en 3D.

## Reglas de Compilación
Cualquier cambio a la extensión C++ requiere compilar usando SCons para el entorno y perfil exacto de Godot 4.7.1-stable.
