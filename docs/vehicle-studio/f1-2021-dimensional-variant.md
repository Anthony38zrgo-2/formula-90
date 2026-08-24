# Variante dimensional híbrida F1 2021/1994

Status: `READY_FOR_HUMAN_GATE`

La variante preserva el F1 1994 como fuente inmutable y aplica un perfil
dimensional reproducible mediante VehicleBuildIR. No publica ni sustituye el
runtime canónico.

## Objetivos aplicados

| Parámetro | Objetivo |
| --- | ---: |
| Wheelbase | 3.640 m |
| Track delantero | 1.635 m |
| Track trasero | 1.575 m |
| Neumático delantero | 0.290 m × 0.660 m |
| Neumático trasero | 0.405 m × 0.670 m |
| Anchura exterior calculada delantera | 1.925 m |
| Anchura exterior calculada trasera | 1.980 m |

El wheelbase es el punto medio del rango típico suministrado de 3.550–3.730 m.
El neumático delantero adopta una proporción híbrida más ligera: mantiene la
llanta de 13 pulgadas, reduce 15 mm de anchura y 10 mm de diámetro frente al
objetivo 2021 puro. La reducción axial es simétrica y conserva el centro del
anclaje y el track delantero.
Los anchos de montaje de llanta de 0.3480 m y 0.4293 m se guardan como datums de
validación y no se confunden con la anchura de la malla del neumático. El límite
de 2.000 m se trata como envolvente; la carrocería 1994 no se escala a ese ancho.

## Evidencia de materialización

- BuildIR: `D7B8397A7C50E035F5DE70ADBDF09A67A05A5039E6B978A76449F1D99AB98686`
- Variante staged: `tools/vehicle_studio/staging/d7b8397a7c50e035f5de70adbdf09a67a05a5039e6b978a76449f1d99ab98686`
- `vehicle.blend`: `9D30E18FA49A89973E66B59C64ADA03341D8DD24FB3CA083279038EA30FF6A68`
- `vehicle.glb`: `4F4CC8C2748D0E5DF1EB5D24FE4D1186181A9376C6CFCEF1295D48CD2A976097`
- `wheel-front.glb`: `152585509F5C37DE18D9F955E6705A02BA38D03F10F74B04C4E21ED23206D56E`
- `wheel-rear.glb`: `6220DBC00EA5153976425B80738D802AFB0213ADF8707A65CDED4D73CB05D51F`
- `vehicle-preview.glb`: `6929741B0A5E3B611CE280387C87BC72C214227E8D90D3B11DE6F5CD5982EB7B`

Blender midió exactamente los siete objetivos. Los contactos delantero y
trasero permanecen en `Y = -0.32298276035985446 m`; la fuente conservó su hash y
la firma topológica no cambió.

## Human gate

En Vehicle Studio, abrir Williams 1994, confirmar el mapping, generar las vistas
CAD, pulsar `Aplicar F1 2021/1994 hybrid (3.640 m)`, compilar y materializar. El
preview interactivo Before/After es la autoridad visual de este gate.
