# Variante dimensional F1 2021 nominal

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
| Neumático delantero | 0.305 m × 0.670 m |
| Neumático trasero | 0.405 m × 0.670 m |
| Anchura exterior calculada delantera | 1.940 m |
| Anchura exterior calculada trasera | 1.980 m |

El wheelbase es el punto medio del rango típico suministrado de 3.550–3.730 m.
Los anchos de montaje de llanta de 0.3480 m y 0.4293 m se guardan como datums de
validación y no se confunden con la anchura de la malla del neumático. El límite
de 2.000 m se trata como envolvente; la carrocería 1994 no se escala a ese ancho.

## Evidencia de materialización

- BuildIR: `9727B3FAA237B579EEFA172ACE2B6A5106D0DC8E9F367A2ECC543BE862904EFF`
- Variante staged: `tools/vehicle_studio/staging/9727b3faa237b579eefa172ace2b6a5106d0dc8e9f367a2ecc543be862904eff`
- `vehicle.blend`: `7C4FFB353083921D6B8EA88279F886C198DB5612DA98D10A36670F11310E435D`
- `vehicle.glb`: `48404B3E4E3099A3CBDE7BB2597A41D612795D66D631E5B053348DA2E96BB3AA`
- `wheel-front.glb`: `8837A803B5739CD1FA8A8F4AD7ACD0FE6F25F1AD295A1CC499F1BCE69C540A5C`
- `wheel-rear.glb`: `6220DBC00EA5153976425B80738D802AFB0213ADF8707A65CDED4D73CB05D51F`
- `vehicle-preview.glb`: `D886EE9601CE1465C99165F921833B2D92590E4EEFF96DF608EB80FB9A752529`

Blender midió exactamente los siete objetivos. Los contactos delantero y
trasero permanecen en `Y = -0.32298276035985446 m`; la fuente conservó su hash y
la firma topológica no cambió.

## Human gate

En Vehicle Studio, abrir Williams 1994, confirmar el mapping, generar las vistas
CAD, pulsar `Aplicar F1 2021 nominal (3.640 m)`, compilar y materializar. El
preview interactivo Before/After es la autoridad visual de este gate.

