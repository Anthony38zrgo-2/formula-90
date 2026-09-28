# Equipo de boxes animado para el F1 2030

Este paquete prepara once riggs esqueléticos animados para la integración en Fuji 76-77. Los modelos GLB están en `poses/`; `pose_manifest.json` relaciona cada puesto con su archivo, sus clips y las ruedas de origen.

- Cuatro portadores llevan únicamente el rin y el neumático de la rueda correspondiente del F1 2030. Cada conjunto está bajo el nodo `CarriedWheel`, separado del cuerpo; disco, pinza y ducto de freno permanecen en el coche.
- Cuatro mecánicos se inclinan hacia el buje y llevan una representación simple de la herramienta de cambio.
- El encargado del gato, el señalero y el encargado de la manguera tienen animaciones y accesorios propios.

Cada GLB conserva la armadura `mixamorig` del personaje y dos clips: `idle_wait` (respiración y transferencia de peso, en bucle mientras el equipo espera) y `service_sequence` (agacharse, alcanzar la rueda, aflojar, encajar y replegarse). La secuencia de servicio hornea desfases temporales por rueda (delantera izquierda al frente, trasera derecha al final) para que los cuatro postes no se muevan sincronizados.

Las poses se generan con Blender 5.2 mediante `tools/pit_crew/generate_animated_pit_crew_rigs.py`. El modelo de origen `source/Racer.fbx` procede de `racer.zip` → `source/Racer.rar` → `Racer.fbx`. El archivo FBX está ignorado por la regla general del repositorio para fuentes 3D; debe conservarse localmente junto al generador. Las ruedas se leen de `game/assets/models/vehicles/f1-2030/`.

`PitCrewVisualController` scrubbea el clip `service_sequence` contra el progreso real del servicio y deja `idle_wait` en bucle durante la espera, con un desfase estable por miembro. El cuerpo, brazos y piernas provienen del esqueleto; el controlador añade el desplazamiento de acercamiento y retirada de cada miembro, acompaña las ruedas con los portadores durante el intercambio y acerca la boquilla del operario de combustible al lateral del coche durante la recarga. El operario retrocede cuando termina el servicio. El coche conserva sus GLB de rueda originales y el controlador vuelve a mostrar rin y neumático al terminar el intercambio.

Los once miembros se cargan e instancian al crear la sesión de carrera compatible y permanecen ocultos hasta que el controlador de boxes los necesita. Al salir del pit lane se ocultan sin destruir sus instancias.
