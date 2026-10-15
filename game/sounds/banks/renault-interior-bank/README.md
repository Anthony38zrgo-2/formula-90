# Prueba Renault R26 interior

Solo las siete fuentes renault_r26_int. Cuatro voces ON/idle y tres OFF. Los archivos upshift/downshift aportan exclusivamente las regiones de bucle declaradas, con frecuencia dominante estable; no reemplazan los efectos de cambio. Los efectos, whine, backfires y limitador originales se conservan.

Conversion a mono PCM16/44100 Hz. Anclas RPM del JSON proporcionado; ajuste minimo para longitud de ciclos enteros. No se dispone del bank original para verificar su hash ni de sus curvas completas. Ventanas de crossfade y objetivo de nivel -17 dBFS son nuevos para esta prueba; pitch proporcional a RPM/referencia, no equivalencia FMOD. Solapamiento de cinco milisegundos para las costuras.

Regenerar: python tools/audio/build_renault_interior_engine_bank.py. Fuentes originales en D:/ASETS/R26_WAV_y_Metadata/R26; el inventario guarda sus hashes.
