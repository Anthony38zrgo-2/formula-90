# Changelog

## V10 3D - 2026-08-05

- Reemplazado el `DirectionalVehicleSprite` del jugador por un GLB 3D real sin modificar handling ni transmisión.
- Añadidos `VehicleVisual3DController` y `VehicleVisual3DConfig` en C++20.
- Separados modelo visual, colisión, sondas de superficie, cámara, audio y reset en `player_car.tscn`.
- Registrados origen, SHA-256, jerarquía, dimensiones, orientación y limitaciones del GLB.
- Añadidos roll, pitch, vibración de superficie y animación opcional de ruedas con fallback seguro.
- Ajustada la cámara arcade al volumen del modelo manteniendo altura y pitch bloqueados.
- Conservados sprites direccionales y herramientas para decoración, placeholder y validación.
- Corregidos el frente `+Z` del GLB mediante rotación visual Y de 180° y la separación entre el plano visible del asfalto y su colisión.
- Corregido el encuadre X de cámara: el objetivo hereda el desplazamiento lateral y mira sutilmente hacia el giro sin expulsar el coche de pantalla.

## Fase 2 - 2026-08-05

- Integrado V10 con nueve vistas reales, hoja direccional y metadatos verificables.
- Añadidas herramientas C++ de sprites, análisis WAV y validación general de assets.
- Generado el banco original `v10_prototype` con Python offline reproducible.
- Implementados crossfade, pitch, filtro, saturación, limitador y cambios en C++.

## 0.1.0 - 2026-08-05

- Bootstrap de Fase 1 con Godot 4.7.1, GDExtension C++20 y campo de pruebas arcade.
- Dirección y agarre lateral corregidos a velocidad media/alta.
- Reversa bloqueada hasta que el coche esté completamente detenido.
- Minimapa estático con límites, postes, salida, posición y orientación del coche.
- Transición avance/reversa con detención estable y frenado activo de la inercia opuesta.
- Controles de conducción movidos a flechas; cambios A/Z y modo automático con la tecla 1.
