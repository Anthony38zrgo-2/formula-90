# Herramientas de sprites

Todas las herramientas visuales son C++20 offline. La referencia principal corresponde al V10 y nunca se modifica: solo se leen copias preservadas bajo `references/`. No inventar, rotar ni deformar vistas inexistentes; el espejo horizontal requiere autorización explícita. Los resultados deben ser PNG RGBA, fondo transparente, nearest-neighbor, centro horizontal y anchor inferior. Cada derivado exige metadatos, validación y pruebas visuales. Toda escritura protege archivos existentes salvo `--force`; toda operación destructiva admite `--dry-run`.

