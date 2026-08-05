# Arquitectura

Fase 2 separa herramientas C++20 para sprites/validación, Python exclusivamente para síntesis WAV offline y DSP/reproducción exclusivamente en la GDExtension. `VehicleDefinition` reúne físicas, presentación y banco; `DirectionalVehicleSprite` consume ángulos y `EngineAudioController` PCM precargado.

La cámara y la presentación direccional se describen en `docs/camera-and-directional-sprites.md`; ambas interpolan después de la física mediante prioridades explícitas.

`GameBootstrap` permanece como raíz y cambia únicamente su escena hija. `MainMenuController` emite intención; el bootstrap decide navegación. `ArcadeCarController` posee movimiento y una `AutomaticTransmission`; `CarPhysicsConfig` contiene ajuste; cámara, sprite, HUD, reset y audio consumen estado sin decidir física. Las escenas son composición declarativa y no contienen lógica GDScript.

`StaticMinimapController` dibuja una proyección fija del campo de pruebas. El mapa no rota ni sigue al coche: una flecha actualizada en C++ representa posición y orientación sobre límites mundiales conocidos.
