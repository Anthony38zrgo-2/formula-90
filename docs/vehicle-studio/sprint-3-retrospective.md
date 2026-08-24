# Sprint 3 — Retrospectiva hasta VS-035

## Funcionó

- BuildIR impide interpretaciones Blender implícitas.
- Factory startup y hashes dobles protegieron las fuentes.
- Separar rueda canónica del chasis permitió probar radio y anchura sin cambios topológicos.
- El reescaneo de outputs detecta paquetes GLB inválidos antes del gate.

## Ajustes realizados

- La materialización pasó de baseline-only a paquete modular completo.
- El determinismo del Blend se evalúa por firma topológica; los GLB sí por bytes.
- El preview Después usa el asset staged por session_id, confinado por el API.

## Deuda posterior al gate

- Instanciar cuatro ruedas en un GLB ensamblado de preview.
- Reabrir y auditar el Blend como parte de VS-036.
- Materiales/livery y deformadores de nariz, alerones, cubremotor y sidepods.
