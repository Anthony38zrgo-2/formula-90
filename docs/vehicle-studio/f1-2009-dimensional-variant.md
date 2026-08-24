# Canónico F1 2009 · Williams FW31

Status: `CANONICAL`

La variante parte directamente del F1-94 canónico remasterizado y aplica un
perfil dimensional reproducible mediante VehicleBuildIR. La variante 2021
anterior queda retirada del runtime y del selector de Vehicle Studio.

Desde esta revisión, `run_f1_94.ps1` inicia este vehículo por defecto. El F1-94
anterior se conserva como variante legacy mediante `-VehicleVariant 1994`.

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
topología y pertenencia UV. Los albedos externos declarados por el asset base
se enlazan por slot semántico y se incrustan en los GLB publicados, por lo que
el resultado no depende de un binder de materiales específico de Godot. La
importación fija `Embed as Basis Universal` para evitar PNG derivados en el
árbol de fuentes.

## Evidencia de materialización

- BuildIR: `BBAB3FD22D24545D56FB2B4492FCA80C888D2BDAB320DDBF1D38F0F395181EFF`
- Variante staged: `tools/vehicle_studio/staging/bbab3fd22d24545d56fb2b4492fca80c888d2bdab320ddbf1d38f0f395181eff`
- `vehicle.blend`: `98FE7F16B8876CE0C902646B7398C2A0236098DA5366B66612CD071A00AC2D82`
- `.blend` publicado: `blender/williams94_wheels_retextured/variants/f1_2009_fw31/F1_2009_fw31.blend`
- `vehicle.glb`: `07B0CF10EDF149A65DD09D42E944B33369CA398D921830948B8A4AD251EED952`
- `wheel-front.glb`: `DF051685E15BFC84B271CF151654B8F2A5EC98B3AE5E3E101C576EB4375DEB27`
- `wheel-rear.glb`: `EEE98C7EFB2C321B5BD4E565281EAB63B3A3A537E8ADD48DE2386446D1283A12`
- `vehicle-preview.glb`: `92B1466C21EE9BC8FC174CA1EBAD1D9EFCED83EDAAE2915D73A4DCFC96081816`
- Contrato de textura: 19/19 materiales de chasis y 4/4 en cada rueda
  contienen `baseColorTexture`; todas las primitivas conservan `TEXCOORD_0`.

## Ejecución

Ejecutar `run_f1_94.ps1` para iniciar el canónico 2009. Para comparar con el
vehículo anterior, ejecutar `run_f1_94.ps1 -VehicleVariant 1994`.
