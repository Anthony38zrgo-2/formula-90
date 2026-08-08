# Estado Actual del Proyecto (F1 2030)

*Última actualización: Agosto 2026*

Este documento rastrea el estado del proyecto a nivel macro. No contiene reglas ni tutoriales, solo el estatus de los subsistemas y deuda técnica.

## 🟢 Sistemas Funcionales
- **Arquitectura GDExtension:** El núcleo en C++20 compila correctamente usando la API de Godot 4.7.
- **Físicas de Vehículo (GEVP):** El coche de jugador inicializado y funcional.
- **Motor Powertrain:** Adaptado recientemente al F1 2026 para comportarse como un **V10 sin ERS**.
  - El peso ha sido ajustado a la normativa 2026 (708kg teórico sin batería).
  - El motor sube a 18,500 RPM.
- **Entorno de Pruebas (`test_field.tscn`):** Implementada pista CSG con geometría corregida para bordillos (Curbs) y muros perimetrales de contención en `X=8`.

## 🟡 Sistemas Parciales o en Desarrollo
- **Pipeline de Audio (V10):** Existen herramientas en Python pero la integración final DSP C++ necesita balanceo.
- **HUD / Telemetría:** Se debe expandir la exposición de variables (ver `telemetry.md`).

## 🔴 Deuda Técnica y Problemas Abiertos
- **Hardcoding de Vehículos:** Las curvas de motor y aerodinámica están harcodeadas dentro de los `.tscn` individuales (ej. `f1_2026_car.tscn`). 
- **Acción requerida (Próximo Milestone):** Mover estas configuraciones hacia una arquitectura orientada a datos (Data-Driven) mediante recursos `.tres` almacenados en `data/engines/`, `data/aero/`, etc.
- **Corrupción de `node_paths`:** Constantes problemas con agentes IA rompiendo referencias de nodos de Godot 4 al sobreescribir escenas. Mantener vigilancia extrema en modificaciones a `.tscn`.
