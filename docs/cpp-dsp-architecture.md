# Arquitectura DSP C++

El flujo es: estado del coche → normalización de RPM → pesos de cinco capas → lectura interpolada/pitch → ganancia de carga → filtro → saturación → limitador → `AudioStreamGeneratorPlayback`.

`EngineLayerMixer` hace crossfade entre capas adyacentes. `EnginePitchProcessor` limita el ratio y responde a reversa. `EngineFilterProcessor` es un low-pass de un polo con cutoff por carga y estado persistente. `EngineSaturator` aplica `tanh`; `EngineLimiter` contiene la salida y neutraliza valores no finitos. Los one-shots usan buffers y cursor independientes.

Los WAV se convierten una vez a vectores preasignados en `_ready`. El procesamiento reutiliza esos buffers, no bloquea, no registra mensajes, no accede a disco y no asigna memoria por sample. La primera implementación llena el ring buffer de Godot desde `_process` a 44.1 kHz, separada estrictamente del generador Python offline.

`EngineAudioConfig` expone banco, RPM, pitch, ganancias, saturación, umbral y tiempos. Las pruebas C++ cubren crossfade, suma, pitch, reversa, filtro persistente/cambio de sample rate, saturación, limitación, NaN y salida sin clipping.
