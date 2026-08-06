# ADR 0002: visual 3D para el vehículo jugable

- Estado: aceptada
- Fecha: 2026-08-05

## Contexto

El jugador se representaba mediante `DirectionalVehicleSprite`, mientras el circuito ya era 3D. El proyecto recibió un GLB V10 generado por Tripo. La simulación arcade y su colisión estable no debían cambiar.

## Decisión

El jugador instancia el GLB mediante una escena contenedora bajo `VehicleVisualRoot`, controlado por `VehicleVisual3DController` y `VehicleVisual3DConfig` en C++20. La raíz física conserva `ArcadeCarController` y un `BoxShape3D` simple. Escala, orientación, offset, ruedas opcionales y movimientos secundarios se aplican exclusivamente al árbol visual.

La escena importada no se edita. La fuente intacta y su hash se conservan en `references`; la copia importable vive en `game/assets`. Los sprites direccionales permanecen disponibles para decoración, placeholder y validación.

El GLB usa `+Z` como frente. La configuración visual aplica una rotación Y de 180° para la convención runtime `-Z`; el offset vertical compensa únicamente el margen físico de contacto. El plano visible del circuito debe coincidir con la cara superior de su colisión.

## Consecuencias

- El coche recibe iluminación y proyecta sombras reales.
- El handling, transmisión y colisión quedan desacoplados del detalle de malla.
- Un GLB con ruedas separadas puede animarse configurando cuatro rutas sin cambiar el controlador.
- El GLB actual es una sola malla; por eso sus ruedas no giran individualmente.
- Roll, pitch y vibración son efectos visuales limitados y suavizados.
- Se añade `RayCast3D` al perfil mínimo de `godot-cpp` para sondas de superficie.

## Alternativas descartadas

- Usar la malla completa como colisión: inestable y costoso para un cuerpo dinámico arcade.
- Escalar `PlayerCar`: también escalaría colisión y espacio físico.
- Editar la escena importada: Godot puede regenerarla y perder cambios.
- Reemplazar la presentación con GDScript: contradice la arquitectura C++ del runtime.
- Eliminar todo el pipeline Sprite3D: rompería decoración y validaciones existentes sin necesidad.
