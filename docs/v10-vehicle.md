# Vehículo V10

V10 es el primer coche definitivo de `formula-90s`. Su referencia intacta está en `references/vehicles/v10/source/v10_reference.png`: PNG de 1000×5720, SHA-256 `ae99f37de9decbc71001cbd8b8e35c321c4b034c564c9d273c6f8d7cc17e79b1`.

La referencia original se usó para generar, con autorización del usuario, un atlas GPT de 16 vistas en estética arcade de 24 bits. Tras retirar el croma, `prepare_sprite` recortó individualmente cada celda, escaló con nearest-neighbor y alineó el contenido en canvas 256×128. La hoja final mide 4096×128 y contiene ambos lados, por lo que el runtime ya no necesita espejado.

`v10_vehicle.tres` enlaza físicas, hoja, metadatos, escala 0.015, offset de suelo 0.08, límite provisional de 285 km/h, 9000 RPM y el banco `v10_prototype`. El runtime calcula el centro vertical desde la altura real del frame para respetar el anchor `(0.5, 1.0)` y evitar que el plano corte el coche.

Para reemplazar la referencia, conserve el nuevo original en `references`, registre dimensiones/hash, ajuste los `crop-rect`, ejecute `prepare_sprite` para cada vista, reconstruya la hoja y termine con `validate_assets`.
