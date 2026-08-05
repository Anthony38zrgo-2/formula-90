# Modelo físico arcade

El controlador descompone velocidad en ejes longitudinal y lateral. Motor/freno actúan longitudinalmente; drag cuadrático y rodadura frenan; el agarre amortigua exponencialmente la velocidad lateral para conservar estabilidad con cualquier framerate. Tras aplicar dirección se recalculan los ejes del coche, dando respuesta clara también por encima de 50 km/h. La dirección interpola entre ajustes de baja/alta velocidad y añade sobreviraje controlado.

La reversa solo se selecciona después de mantener la velocidad horizontal total menor o igual a `0.02 m/s` durante `0.18 s`; durante esa ventana la velocidad se fija exactamente en cero. Mientras exista movimiento hacia delante, abajo actúa exclusivamente como freno. Si el coche rueda hacia atrás y se pulsa acelerar, primero frena activamente la inercia, espera la detención estable y luego engrana primera.

La transmisión arranca en modo automático. `1` alterna automático/manual; `A` sube y `Z` baja dentro de las seis marchas hacia delante. La selección de reversa permanece gestionada por la transición de dirección para impedir cambios peligrosos en movimiento. No pretende simulación de neumático o suspensión.
