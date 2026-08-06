# Proyecto Godot

Flujo: `Bootstrap` persistente carga `MainMenu` o `TestField`. No renombrar nodos documentados sin actualizar C++. Input Map: `accelerate`, `brake`, `steer_left`, `steer_right`, `strong_brake`, `shift_up`, `shift_down`, `toggle_automatic`, `reset_vehicle`, `ui_back_to_menu`.

Resolución interna 640x360, escalado canvas_items. El jugador usa un GLB real bajo `VehicleVisualRoot`; los Sprite3D direccionales quedan para decoración y validación, con nearest y sin mipmaps. Física, colisión y presentación permanecen separadas. Escenas/resources declaran composición y valores; lógica en C++. Recursos `.tres` en snake_case y con tipo explícito.

No editar directamente escenas importadas desde GLB. Instanciarlas mediante `game/scenes/vehicles/<id>/` y aplicar escala/orientación únicamente en el nodo visual configurado.
