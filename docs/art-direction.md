# Dirección de arte

Base 640×360 escalable, contraste alto, paleta saturada y siluetas legibles. El jugador usa un modelo 3D real separado de su colisión; objetos laterales, público, marshals y vegetación pueden seguir usando sprites sobre geometría invisible.

## V10 3D

El GLB actual tiene una malla blanca opaca, sin texturas ni UV. Se conserva su material importado y se ilumina mediante el ambiente y la luz direccional de la pista. El modelo recibe iluminación y proyecta sombra. La escala visual `4.0` produce un volumen aproximado de `1.77 × 1.05 × 3.99 m` y mantiene las ruedas en Y=0.

Roll y pitch son moderados y comunican peso sin alterar la física. La vibración solo aparece al detectar pianos o grava etiquetados. No se añaden blur, balanceo constante ni movimiento vertical de cámara.

## Sprites 2.5D

El atlas V10 de 16 vistas y `DirectionalVehicleSprite` se conservan como tecnología para decoración y validación, pero ya no se instancian en el jugador. Los sprites mantienen estética prerenderizada de 24 bits, alpha limpio, filtro nearest y ausencia de mipmaps.

No mezclar un sprite y el modelo 3D para representar simultáneamente el mismo vehículo.
