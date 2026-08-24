# VS-033..VS-035 — Human gate del exportador GLB

Status: READY_FOR_HUMAN_GATE

## Resultado funcional

- Radio delantero/trasero independiente con contacto en suelo.
- Altura del chasis interpolada entre ejes y rake delta reportado.
- Anchura delantera/trasera con políticas centered, inboard_fixed y outboard_fixed.
- Wheelbase y tracks combinables con neumáticos en un mismo BuildIR.
- Salidas staged content-addressed:
  - vehicle.blend
  - vehicle.glb
  - wheel-front.glb
  - wheel-rear.glb
  - materialization-report.json
- Panel Before/After: fuente original frente a vehicle.glb staged.
- Ninguna publicación a runtime.

## Variante Williams del gate

BuildIR: 04FEF03BCD6BE88571B4BD27C0C9B552D653C7D8B4532E8026E09AFCFCFB3421

Cambios combinados:

- wheelbase +5 %
- front track +3 %
- rear track -2 %
- front tire radius +10 %
- rear tire radius -5 %
- front tire width +15 %, inboard_fixed
- rear tire width -10 %, outboard_fixed

Resultados:

- contacto delantero: 0.0 m
- contacto trasero: 0.0 m
- altura delantera delta: +0.031695619536451 m
- altura trasera delta: -0.016450466267759978 m
- rake delta: -0.015699724020136745 rad
- chassis GLB: 60EA2F989E35D3F031A4FD9DA7009DC9C9973AF4496C185207ABE053D31ECE21
- front wheel GLB: 96BDE326F61557092105F17B3F3E6C370CD95151F0453F6EED63C7E645859E34
- rear wheel GLB: 542B043D28095D56D17811DB00780758E31478BC84B173140CDF335AC54BE894

## Determinismo y límites

Los GLB son byte-deterministas entre staging roots independientes. El Blend conserva firma topológica pero su hash cambia al contener la ruta absoluta de guardado. Topología, polígonos y pertenencia UV se conservan. Los GLB de ruedas son canónicos separados; el preview comparativo actual muestra el chasis staged y no instancia visualmente las cuatro ruedas.

## Rollback

Eliminar únicamente la carpeta content-addressed bajo tools/vehicle_studio/staging. Las fuentes Williams no se modifican.


## Corrección de integración posterior

- Restaurado POST /api/build/materialize y la propagación de width_policies.
- Plano de suelo Williams medido: -0.32298276035985446 m.
- Nuevo vehicle-preview.glb con chasis y cuatro ruedas instanciadas.
- Anclajes: X = +/-track/2, Z = +/-wheelbase/2, Y = ground + radius.
- Raíces verificadas: WHEEL_FL, WHEEL_FR, WHEEL_RL y WHEEL_RR.
- BuildIR de evidencia: A980A20709D44794F3649B54F8408D09646F6746FB3AB11E74291BB2F7612AE6.
- SHA-256 vehicle-preview.glb: 34A1672FC87774D64286CD98BA9FA8C44720972E3658F1B2DEAD29C553B24038.
- Suite final: 62 Python, 6 Vue, vue-tsc y Vite OK.
