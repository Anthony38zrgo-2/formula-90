class_name BackgroundPreset
extends Resource

## Define la configuracion completa de un preset de background (Formula-90).
## El skybox (si existe) esta desacoplado de las capas parallax.
##
## JSON parsing mirrors `game/crates/skybox-engine/src/preset.rs`.
## Same inputs must produce same outputs. Rust is the authority.

@export var id: StringName = &""
@export var display_name: String = ""
@export var skybox: BackgroundSkyboxConfig
@export var layers: Array[BackgroundLayerConfig] = []


static func from_dict(dict: Dictionary) -> BackgroundPreset:
	var preset := BackgroundPreset.new()
	preset.id = StringName(dict.get("id", ""))
	preset.display_name = str(dict.get("display_name", preset.id))
	
	var raw_skybox = dict.get("skybox", null)
	if raw_skybox is Dictionary:
		preset.skybox = BackgroundSkyboxConfig.from_dict(raw_skybox)
	
	var raw_layers = dict.get("layers", [])
	if raw_layers is Array:
		for raw_layer in raw_layers:
			if raw_layer is Dictionary:
				preset.layers.append(BackgroundLayerConfig.from_dict(raw_layer))
	return preset


static func load_from_json_file(file_path: String) -> BackgroundPreset:
	if not FileAccess.file_exists(file_path):
		push_error("BackgroundPreset: No se encontro el archivo en la ruta '%s'." % file_path)
		return null
	
	var file := FileAccess.open(file_path, FileAccess.READ)
	if file == null:
		push_error("BackgroundPreset: Error al abrir el archivo '%s' (Error %d)." % [file_path, FileAccess.get_open_error()])
		return null
	
	var text := file.get_as_text()
	var json := JSON.new()
	var parse_result := json.parse(text)
	if parse_result != OK:
		push_error("BackgroundPreset: Error de parseo JSON en '%s' (Linea %d: %s)." % [file_path, json.get_error_line(), json.get_error_message()])
		return null
	
	var data = json.get_data()
	if not (data is Dictionary):
		push_error("BackgroundPreset: La raiz del JSON en '%s' debe ser un objeto/diccionario." % file_path)
		return null
	
	return from_dict(data as Dictionary)


func to_dict() -> Dictionary:
	var layers_data: Array[Dictionary] = []
	for layer in layers:
		if layer != null:
			layers_data.append(layer.to_dict())
	return {
		"id": String(id),
		"display_name": display_name,
		"skybox": skybox.to_dict() if skybox != null else {},
		"layers": layers_data
	}


func get_layers_sorted_by_depth() -> Array[BackgroundLayerConfig]:
	var sorted_layers := layers.duplicate()
	sorted_layers.sort_custom(func(a: BackgroundLayerConfig, b: BackgroundLayerConfig) -> bool:
		return a.depth < b.depth
	)
	return sorted_layers


func get_layer_by_id(layer_id: StringName) -> BackgroundLayerConfig:
	for layer in layers:
		if layer != null and layer.id == layer_id:
			return layer
	return null
