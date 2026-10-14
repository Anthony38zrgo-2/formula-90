# Ferrari 2008: candidato V8 conservado

El manifest usa diez voces de las ramas exteriores traseras ON/OFF originales. Las anclas y ventanas RPM se recuperan por CTRL.target_guid -> WAIB.instrument_guid; las curvas de acelerador se recuperan por bus. Caja, whine, backfires, limitador y politica de eventos son copias exactas del banco V10 previo. Commons permanece separado.

Limites: interpolacion lineal reconstruida, acelerador interpretado como decibelios y pitch proporcional a RPM/ancla; no se afirma equivalencia exacta con FMOD. No se reproducen automatizaciones espaciales ni de pitch originales. Se selecciona una perspectiva exterior trasera mono, sin conmutacion de camara. Las fuentes interiores y frontales permanecen disponibles. El solapamiento de cinco milisegundos de los loops preparados es nuevo, no metadata original.

Regenerar con `python tools/audio/build_ferrari_recorded_engine_bank.py`. Usa las fuentes WAV verificadas y los enlaces recuperados incluidos; si el bank original esta disponible, vuelve a verificar los enlaces binarios. Metadatos originales: original_event_metadata.json. Grafo y limites: recovered_automation.json. La activacion modifica solamente profiles/geometric/audio.json y el soporte de automatizacion de motor del sampler.
