# VS-031 — Blender materializer scaffold

Status: READY_FOR_HUMAN_GATE

## Resultado

- Materializador Python con preflight completo antes de crear staging.
- Worker Blender ejecutado con factory startup y selección semántica nula.
- Fuente GLB concreta y su SHA-256 propagados desde scan a VehicleDocument y BuildIR.
- Salida content-addressed: vehicle.blend y materialization-report.json.
- Las operaciones geométricas cambiadas aún no implementadas fallan explícitamente con MATERIALIZER_UNSUPPORTED_CHANGED_OPERATION.

## Evidencia automatizada

- Suite: 61 pruebas OK.
- Dos ejecuciones Blender independientes produjeron la misma firma topológica.
- Hash del GLB fuente idéntico antes y después.
- Hash incorrecto falla antes de crear staging.
- Cambio no soportado falla antes de crear staging.

## Límite deliberado

VS-031 prueba la frontera segura Python→Blender y materializa solamente el baseline. No afirma que los sliders deformen geometría. Wheelbase/anchura global pertenecen a VS-032; neumáticos y contacto con suelo a VS-033/VS-034.

## Rollback

Eliminar materializer.py, materialize_build_ir.py y sus pruebas. No existe publicación ni mutación del Williams fuente.
