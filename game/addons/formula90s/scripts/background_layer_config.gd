class_name BackgroundLayerConfig
extends Resource

## Configuracion individual para una capa de background multicapa (Formula-90)

enum ParallaxMode { PARALLAX, FOLLOW }

@export var id: StringName = &""
@export var texture_path: String = ""
@export var depth: int = 0
@export var parallax_mode: int = ParallaxMode.PARALLAX
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

	var mode_str := str(dict.get("parallax_mode", "parallax")).to_lower()
	layer.parallax_mode = ParallaxMode.FOLLOW if mode_str == "follow" else ParallaxMode.PARALLAX

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
		"parallax_mode": "follow" if parallax_mode == ParallaxMode.FOLLOW else "parallax",
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


## Calcula el pixel_size minimo para que un Sprite3D a [param distance_z] cubra
## el frustum de una camara con [param fov_deg] grados de FOV horizontal y un
## viewport de [param viewport_size] pixeles. El [param texture_size] es la
## resolucion de la textura del sprite.
##
## La formula deriva de: sprite_world_size = texture_size * pixel_size
## y visible_world_size = 2 * |distance_z| * tan(fov_h/2)
## Queremos sprite_world_size >= visible_world_size * coverage_factor.
static func compute_pixel_size_from_camera(
	fov_deg: float,
	distance_z: float,
	viewport_size: Vector2i,
	texture_size: Vector2i,
	coverage_factor: float = 1.2
) -> float:
	if texture_size.y <= 0 or viewport_size.y <= 0 or fov_deg <= 0.0:
		return 0.5
	var fov_h_rad := deg_to_rad(fov_deg)
	var aspect := float(viewport_size.x) / float(viewport_size.y)
	var fov_v_rad := 2.0 * atan(tan(fov_h_rad * 0.5) / aspect)
	var visible_h := 2.0 * absf(distance_z) * tan(fov_v_rad * 0.5)
	var needed_px := visible_h * coverage_factor / float(texture_size.y)
	# Redondear a 0.05 para pixel_snap limpio
	return snappedf(maxf(needed_px, 0.1), 0.05)
