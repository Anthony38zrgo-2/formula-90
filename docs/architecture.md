# Arquitectura

`GameBootstrap` permanece como raíz y cambia únicamente su escena hija. `MainMenuController` emite intención; el bootstrap decide navegación. `ArcadeCarController` posee movimiento y una `AutomaticTransmission`; `CarPhysicsConfig` contiene ajuste; cámara, sprite, HUD, reset y audio consumen estado sin decidir física. Las escenas son composición declarativa y no contienen lógica GDScript.

