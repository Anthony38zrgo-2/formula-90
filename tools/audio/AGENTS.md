# Audio offline V10

Python se usa únicamente para generar samples WAV offline y nunca se invoca desde Godot, el runtime o un export. El banco inicial es `v10_prototype`: mono, 44.1 kHz, semilla reproducible, normalización moderada y prevención de clipping/NaN/infinitos/DC. No imitar ni copiar motores, equipos, temporadas o grabaciones reales. Toda reproducción, análisis usado por el juego, mezcla, crossfade, pitch, filtros, saturación y limitación pertenece exclusivamente a C++.

