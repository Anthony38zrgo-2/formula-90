# Validación de assets

- Estas herramientas son C++20 independientes del runtime y nunca invocan Python.
- `analyze_audio` analiza WAV PCM; `validate_assets` valida el banco y la hoja V10.
- Los errores devuelven código distinto de cero y las advertencias se identifican por separado.
- Los fixtures de pruebas deben ser originales, pequeños y generados por el propio test.
