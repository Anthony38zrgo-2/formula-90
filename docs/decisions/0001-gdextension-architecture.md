# ADR 0001: GDExtension C++

Estado: aceptada. Godot 4.7.1 ejecuta un runtime C++20 mediante `godot-cpp`, sin recompilar el motor. Escenas/resources quedan declarativos y herramientas futuras quedan fuera del runtime. Esto conserva iteración de Godot y tipos nativos comprobables.

