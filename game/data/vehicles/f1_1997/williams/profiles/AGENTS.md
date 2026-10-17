# profiles — configuraciones físicas por circuito

Ámbito: un directorio por circuito con los trece dominios que resuelve el motor nativo.
Consumidores: `game/scripts/vehicle/vehicle_profile_configuration.gd` y el resolvedor de `vehicle_physics_engine`.
Reglas: `vehicle.json` declara los dominios; cada dominio conserva el esquema del contrato y las unidades SI. Cualquier valor heredado del perfil base debe estar justificado en `profile_provenance.json`. La resolución debe pasar antes de declarar el perfil listo.
Subcarpetas: cada circuito documenta su reglaje en su propio `AGENTS.md`.
