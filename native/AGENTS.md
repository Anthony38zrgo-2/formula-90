# Runtime C++

Usar C++20 y registrar cada clase en `register_types.cpp`. PascalCase para clases, snake_case para archivos y métodos. Un header público bajo `include/formula90s/` y un source paralelo bajo `src/`. Usar `Ref<>`, `Object::cast_to`, `memnew/memdelete` y ownership de Godot; no `new/delete` crudos para objetos Godot. Exportar parámetros ajustables y validar rangos. Mantener clases pequeñas, logs con prefijo `[formula90s]` y pruebas para lógica determinista. No usar GDScript como parche de errores del runtime.

