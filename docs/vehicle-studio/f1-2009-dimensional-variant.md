# Variante dimensional F1 2009 · Williams FW31

Status: `READY_FOR_HUMAN_GATE`

La variante parte directamente del F1-94 canónico remasterizado y aplica un
perfil dimensional reproducible mediante VehicleBuildIR. La variante 2021
anterior queda retirada del runtime y del selector de Vehicle Studio.

## Fuentes y criterio

- [FIA, *2009 Formula One Technical Regulations*](https://argent.fia.com/web/fia-public.nsf/E280B4702B22A1E9C125753C0056FF76/$FILE/Formula%20One%20Technical%20regulations%20Sum.pdf), artículos 3.3 y 12.4: anchura
  total máxima de 1.800 m, diámetro máximo de rueda seca de 0.660 m, anchuras
  completas de 0.305–0.355 m delante y 0.365–0.380 m detrás, y diámetro de
  asiento de talón de 0.328–0.332 m.
- [Ficha técnica de lanzamiento del Williams FW31, publicada por Williams F1](https://m.f1network.net/main/s107/st138961.htm): wheelbase de 3.100 m, dimensiones
  exteriores de 4.800 × 1.800 × 0.950 m y ruedas completas de 0.350 m delante
  y 0.375 m detrás.

El reglamento no fijaba el wheelbase. Los tracks no fueron publicados en la
ficha del FW31, por lo que se derivan de forma explícita para que rueda completa
y track cierren exactamente la envolvente FIA de 1.800 m:
`track = anchura total - anchura de rueda completa`.

## Objetivos aplicados

| Parámetro | Objetivo | Procedencia |
| --- | ---: | --- |
| Wheelbase | 3.100 m | Williams FW31 |
| Track delantero | 1.450 m | Inferido: 1.800 − 0.350 |
| Track trasero | 1.425 m | Inferido: 1.800 − 0.375 |
| Neumático delantero | 0.350 m × 0.660 m | FW31 + máximo FIA seco |
| Neumático trasero | 0.375 m × 0.660 m | FW31 + máximo FIA seco |
| Anchura exterior calculada | 1.800 m | Envolvente FIA |

La escala axial es simétrica alrededor de cada anclaje. Los centros de las
ruedas no cambian con el ancho y ambos ejes mantienen contacto en
`Y = -0.32298276035985446 m`. La fuente F1-94 no se modifica, y se conservan
topología y pertenencia UV.

## Evidencia de materialización

- BuildIR: `0DDB18624C47C2EF97059B844F3BBFE821E99059A4F2912EADCECAF6239960CC`
- Variante staged: `tools/vehicle_studio/staging/0ddb18624c47c2ef97059b844f3bbfe821e99059a4f2912eadcecaf6239960cc`
- `vehicle.blend`: `B2ED167E344816B95C612F56D326931C5D17815B23D486B25A1E0666B3BAA815`
- `vehicle.glb`: `08A3A326C2389D960A73CC6356579319CE9F28A8895E6D0ED68BF94A1AD619D2`
- `wheel-front.glb`: `5288BBCE6076DC9FB314D2B18FC48C81F37B44B756B26BD65A8ADEF978BFF4DD`
- `wheel-rear.glb`: `114C68963217460E78543BD3616098998D8D400BCF2EFAACF7381673CFF9A49A`
- `vehicle-preview.glb`: `0C275BF4AB0A6BB66620DD32EE1C3E1C0C458EB60C69F71829177A6308EBE670`

## Human gate

Ejecutar `run_f1_94.ps1 -VehicleVariant 2009` y verificar proporciones,
contacto con el suelo, dirección y comportamiento del tren delantero antes de
considerar la variante aprobada.
