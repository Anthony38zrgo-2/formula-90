# Runtime C++

El DSP y el procesamiento/selección direccional de sprites son exclusivamente C++. El runtime no puede invocar Python. El audio debe precargar samples y metadatos, reutilizar buffers, mantener el estado de filtros y evitar asignaciones, bloqueos, disco y logs en su bucle de procesamiento. La mezcla V10 usa crossfade de capas y debe informar claramente assets ausentes o incompatibles.

Usar C++20 y registrar cada clase en `register_types.cpp`. PascalCase para clases, snake_case para archivos y métodos. Un header público bajo `include/formula90s/` y un source paralelo bajo `src/`. Usar `Ref<>`, `Object::cast_to`, `memnew/memdelete` y ownership de Godot; no `new/delete` crudos para objetos Godot. Exportar parámetros ajustables y validar rangos. Mantener clases pequeñas, logs con prefijo `[formula90s]` y pruebas para lógica determinista. No usar GDScript como parche de errores del runtime.

`VehicleVisual3DController` consume la pose interpolada del coche sin escribir física. Escala, offset, roll, pitch, vibración y ruedas opcionales pertenecen exclusivamente a la presentación; nodos ausentes deben degradar funcionalidad sin invalidar la escena.
