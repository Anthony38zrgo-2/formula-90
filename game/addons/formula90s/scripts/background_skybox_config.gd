class_name BackgroundSkyboxConfig
extends Resource

## Configuracion de un skybox desacoplado que sigue la camara (Formula-90).
## El skybox es independiente de las capas parallax: una sola textura que
## llena el frustum en todo momento y rota con la camara.

@export var texture_path: String = ""
@export var distance: float = 800.0
@export var pixel_size: float = 0.0


static func from_dict(dict: Dictionary) -> BackgroundSkyboxConfig:
	var cfg := BackgroundSkyboxConfig.new()
	cfg.texture_path = str(dict.get("texture", dict.get("texture_path", "")))
	cfg.distance = float(dict.get("distance", 800.0))
	cfg.pixel_size = float(dict.get("pixel_size", 0.0))
	return cfg


func to_dict() -> Dictionary:
	return {
		"texture": texture_path,
		"distance": distance,
		"pixel_size": pixel_size
	}
