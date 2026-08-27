# Shared formats

Definiciones de formatos que ya tienen dos o más consumidores importantes.

Subformatos vigentes:

- `audio_bank/`: manifiesto mínimo compartido por tooling Python, runtime Rust
  y el adaptador Godot.

Reglas:

- no crear schemas para llenar la estructura objetivo;
- productor y consumidores pueden evolucionar juntos mientras el formato sea
  interno y volátil;
- añadir versionado o compatibilidad histórica únicamente cuando aporte un
  beneficio real;
- configuración concreta y datos de gameplay no se convierten automáticamente
  en schemas.

Arquitectura canónica: `docs/decoupling-blueprint.md`.
