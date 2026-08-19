class_name BackgroundSkyboxConfig
extends Resource

## Configuracion de un skybox desacoplado que sigue la camara (Formula-90).
## Puede usar una textura estática o un degradado procedural según hora del día.

@export var mode: String = "texture"
@export var texture_path: String = ""
@export var distance: float = 800.0
@export var pixel_size: float = 0.0
@export var gradient: Dictionary = {}
@export var time_of_day: float = 11.0


static func from_dict(dict: Dictionary) -> BackgroundSkyboxConfig:
	var cfg := BackgroundSkyboxConfig.new()
	cfg.mode = str(dict.get("mode", "texture"))
	cfg.texture_path = str(dict.get("texture", dict.get("texture_path", "")))
	cfg.distance = float(dict.get("distance", 800.0))
	cfg.pixel_size = float(dict.get("pixel_size", 0.0))
	cfg.time_of_day = float(dict.get("time_of_day", 11.0))
	var raw_gradient = dict.get("gradient", null)
	if raw_gradient is Dictionary:
		cfg.gradient = raw_gradient.duplicate()
	return cfg


func to_dict() -> Dictionary:
	var d := {
		"mode": mode,
		"distance": distance,
		"pixel_size": pixel_size,
		"time_of_day": time_of_day
	}
	if mode == "texture":
		d["texture"] = texture_path
	elif mode == "gradient":
		d["gradient"] = gradient.duplicate()
	return d
