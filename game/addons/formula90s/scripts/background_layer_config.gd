class_name BackgroundLayerConfig
extends Resource

## Configuracion individual para una capa parallax de background (Formula-90).
## El skybox ya no es una capa: es un componente desacoplado (BackgroundSkybox).

@export var id: StringName = &""
@export var texture_path: String = ""
@export var depth: int = 0
@export var parallax_x: float = 0.0
@export var parallax_y: float = 0.0
@export var scale: Vector2 = Vector2.ONE
@export var offset: Vector2 = Vector2.ZERO
@export var repeat_x: bool = false
@export var repeat_y: bool = false
@export var pixel_snap: bool = true
@export var distance_z: float = -800.0
@export var pixel_size: float = 0.5


static func from_dict(dict: Dictionary) -> BackgroundLayerConfig:
	var layer := BackgroundLayerConfig.new()
	layer.id = StringName(dict.get("id", ""))
	layer.texture_path = str(dict.get("texture", dict.get("texture_path", "")))
	layer.depth = int(dict.get("depth", 0))
	layer.parallax_x = float(dict.get("parallax_x", 0.0))
	layer.parallax_y = float(dict.get("parallax_y", 0.0))
	
	var raw_scale = dict.get("scale", 1.0)
	if raw_scale is float or raw_scale is int:
		layer.scale = Vector2(float(raw_scale), float(raw_scale))
	elif raw_scale is Array and raw_scale.size() >= 2:
		layer.scale = Vector2(float(raw_scale[0]), float(raw_scale[1]))
	elif raw_scale is Dictionary:
		layer.scale = Vector2(float(raw_scale.get("x", 1.0)), float(raw_scale.get("y", 1.0)))
	else:
		layer.scale = Vector2.ONE

	layer.offset = Vector2(float(dict.get("offset_x", 0.0)), float(dict.get("offset_y", 0.0)))
	layer.repeat_x = bool(dict.get("repeat_x", false))
	layer.repeat_y = bool(dict.get("repeat_y", false))
	layer.pixel_snap = bool(dict.get("pixel_snap", true))
	
	if dict.has("distance_z"):
		layer.distance_z = float(dict["distance_z"])
	else:
		layer.distance_z = -800.0 + (layer.depth * 100.0)
		
	layer.pixel_size = float(dict.get("pixel_size", 0.5))
	return layer


func to_dict() -> Dictionary:
	return {
		"id": String(id),
		"texture": texture_path,
		"depth": depth,
		"parallax_x": parallax_x,
		"parallax_y": parallax_y,
		"scale": {"x": scale.x, "y": scale.y},
		"offset_x": offset.x,
		"offset_y": offset.y,
		"repeat_x": repeat_x,
		"repeat_y": repeat_y,
		"pixel_snap": pixel_snap,
		"distance_z": distance_z,
		"pixel_size": pixel_size
	}
