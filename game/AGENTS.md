# Proyecto Godot

Flujo: `Bootstrap` persistente carga `MainMenu` o `TestField`. No renombrar nodos documentados sin actualizar C++. Input Map GEVP: `Throttle`, `Brakes`, `Steer Left`, `Steer Right`, `Handbrake`, `Clutch`, `Toggle Transmission`, `Shift Up`, `Shift Down`. Acciones propias: `ui_back_to_menu`, `aid_1`..`aid_5`, `ShowDebug`, `DebugNext`, `DebugPrevious`.

Resolución interna 640x360, escalado canvas_items. La física del vehículo usa GEVP (GDScript). El jugador usa un GLB real bajo el nodo del vehículo; los Sprite3D direccionales quedan para decoración y validación, con nearest y sin mipmaps. Física, colisión y presentación permanecen separadas. Escenas/resources declaran composición y valores. Recursos `.tres` en snake_case y con tipo explícito.

No editar directamente escenas importadas desde GLB. Instanciarlas mediante `game/scenes/vehicles/<id>/` y aplicar escala/orientación únicamente en el nodo visual configurado.
