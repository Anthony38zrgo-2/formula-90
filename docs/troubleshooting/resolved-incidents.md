# Resolved Incidents (Postmortems)

Repositorio de conocimiento técnico sobre bugs graves, esotéricos o fuertemente arraigados en el engine, ya resueltos.
Evita repetir errores leyendo estos incidentes.

---

## Incident 001: Pérdida Silenciosa de Enlaces a Nodos en Godot 4 (node_paths)

### Symptoms
Scripts como el `VehicleController` o el HUD dejaban de recibir inputs, los sistemas de ayudas devolvían Null Exceptions en runtime, o fallaban referenciando el `DrivingAids`. El árbol parecía correcto a simple vista en el IDE.

### Root cause
En Godot 4, cuando exportas una variable de tipo Node (`@export var my_node: Node`) en un script, el motor de escenas requiere un atributo especial de metadatos en formato texto dentro del `.tscn`: `node_paths=PackedStringArray("my_node")`. 

### Failed approaches
- Añadir getters/setters redundantes.
- Intentar usar `get_node()` en `_ready()` parcheando el código.
- Asumir que la inyección de dependencias estaba mal diseñada.

### Correct fix
Restaurar explícitamente `node_paths=PackedStringArray(...)` en la declaración del nodo principal del archivo `.tscn` cuando se hace un diff o una modificación del texto crudo de la escena.

### Lesson
Los agentes AI tienen la costumbre de regenerar bloques de código `.tscn` basándose en el script original, olvidando los atributos ocultos de metadata inyectados por el inspector de Godot. **Cualquier edición a un `.tscn` exportando nodos exige validación del array `node_paths`.**
