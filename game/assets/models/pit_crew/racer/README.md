# Equipo de boxes estático para el F1 2030

Este paquete prepara once poses independientes para la primera integración en Fuji 76-77. Los modelos GLB están en `poses/`; `pose_manifest.json` relaciona cada puesto con su archivo y las ruedas de origen.

- Cuatro portadores llevan únicamente el rin y el neumático de la rueda correspondiente del F1 2030. Cada conjunto está bajo el nodo `CarriedWheel`, separado del cuerpo; disco, pinza y ducto de freno permanecen en el coche.
- Cuatro mecánicos se inclinan hacia el buje y llevan una representación simple de la herramienta de cambio.
- El encargado del gato, el señalero y el encargado de la manguera tienen poses y accesorios estáticos propios.

Las poses se generan con Blender 5.2 mediante `tools/pit_crew/generate_static_pit_crew_poses.py`. El modelo de origen `source/Racer.fbx` procede de `racer.zip` → `source/Racer.rar` → `Racer.fbx`. El archivo FBX está ignorado por la regla general del repositorio para fuentes 3D; debe conservarse localmente junto al generador. Las ruedas se leen de `game/assets/models/vehicles/f1-2030/`.

Los GLB exportados son poses estáticas. `PitCrewVisualController` anima los desplazamientos y las herramientas de los mecánicos durante el cambio, acompaña las ruedas con los portadores y acerca la boquilla del operario de combustible al lateral del coche durante la recarga. El operario retrocede cuando termina el servicio. El coche conserva sus GLB de rueda originales y el controlador vuelve a mostrar rin y neumático al terminar el intercambio.

Los once miembros se cargan e instancian al crear la sesión de carrera compatible y permanecen ocultos hasta que el controlador de boxes los necesita. Al salir del pit lane se ocultan sin destruir sus instancias.
