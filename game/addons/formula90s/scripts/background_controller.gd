class_name BackgroundController
extends Node3D

## Controlador de Background Multicapa desacoplado para circuitos (Formula-90)
## Gestiona la carga de presets, ciclo de vida de capas, seguimiento de camara y calculo de parallax.

@export var is_active: bool = true
@export var debug_mode: bool = false

var _active_preset: BackgroundPreset
var _camera: Camera3D
var _layer_instances: Array[BackgroundLayerInstance] = []
var _layer_map: Dictionary = {} # StringName -> BackgroundLayerInstance


func _ready() -> void:
	_validate_visual_only_hierarchy()
	_resolve_current_camera()
	_follow_active_camera()
	_update_layers_parallax()


func _process(_delta: float) -> void:
	if not is_active:
		return
	
	_resolve_current_camera()
	_follow_active_camera()
	_update_layers_parallax()


## Carga y aplica un preset de background. Valida antes de instanciar.
func load_preset(preset: BackgroundPreset) -> bool:
	if preset == null:
		push_error("BackgroundController: Se intento cargar un preset nulo.")
		return false
	
	var validation := BackgroundValidator.validate_preset(preset)
	if not validation.is_valid:
		push_error("BackgroundController: Fallo la validacion del preset '%s':\n%s" % [preset.id, validation.get_error_summary()])
		return false
	
	clear_layers()
	_active_preset = preset
	
	var sorted_layers := preset.get_layers_sorted_by_depth()
	for layer_cfg in sorted_layers:
		var instance := BackgroundLayerInstance.new()
		var success := instance.setup_layer(layer_cfg)
		if not success:
			push_error("BackgroundController: Fallo al instanciar la capa '%s'." % layer_cfg.id)
			instance.queue_free()
			continue
		
		add_child(instance)
		_layer_instances.append(instance)
		_layer_map[layer_cfg.id] = instance
	
	_validate_visual_only_hierarchy()
	_update_layers_parallax()
	return true


## Carga un preset directamente desde un archivo JSON externo
func load_preset_from_file(file_path: String) -> bool:
	var preset := BackgroundPreset.load_from_json_file(file_path)
	if preset == null:
		push_error("BackgroundController: No se pudo cargar el preset desde '%s'." % file_path)
		return false
	return load_preset(preset)


## Limpia todas las instancias de capas activas
func clear_layers() -> void:
	for inst in _layer_instances:
		if is_instance_valid(inst):
			inst.queue_free()
	_layer_instances.clear()
	_layer_map.clear()
	_active_preset = null


## Enlaza una camara explicita como fuente de movimiento
func set_camera_source(cam: Camera3D) -> void:
	_camera = cam
	_follow_active_camera()
	_update_layers_parallax()


func set_enabled(enabled: bool) -> void:
	is_active = enabled
	visible = enabled


func is_enabled() -> bool:
	return is_active


func get_active_preset() -> BackgroundPreset:
	return _active_preset


func get_layer_instances() -> Array[BackgroundLayerInstance]:
	return _layer_instances


func get_layer_instance_by_id(layer_id: StringName) -> BackgroundLayerInstance:
	return _layer_map.get(layer_id, null)


func get_debug_info() -> Dictionary:
	var layer_info: Array[Dictionary] = []
	for inst in _layer_instances:
		if is_instance_valid(inst) and inst.layer_config != null:
			layer_info.append({
				"id": String(inst.layer_config.id),
				"depth": inst.layer_config.depth,
				"parallax_x": inst.layer_config.parallax_x,
				"parallax_y": inst.layer_config.parallax_y,
				"position": inst.position,
				"visible": inst.visible
			})
	return {
		"active_preset": String(_active_preset.id) if _active_preset != null else "none",
		"is_active": is_active,
		"camera_valid": is_instance_valid(_camera),
		"layers_count": _layer_instances.size(),
		"layers": layer_info
	}


func _resolve_current_camera() -> void:
	if not is_instance_valid(_camera):
		var viewport := get_viewport()
		if viewport != null:
			var current_cam := viewport.get_camera_3d()
			if current_cam != null:
				_camera = current_cam


func _follow_active_camera() -> void:
	if not is_inside_tree() or not is_instance_valid(_camera) or not _camera.is_inside_tree():
		return
	# Sigue la posicion de la camara sin heredar su rotacion en el nodo raiz
	global_position = Vector3(_camera.global_position.x, global_position.y, _camera.global_position.z)


func _update_layers_parallax() -> void:
	if not is_inside_tree() or not is_instance_valid(_camera) or not _camera.is_inside_tree() or _layer_instances.is_empty():
		return
	
	# Extraer rotacion yaw y pitch de la camara
	var cam_euler := _camera.global_transform.basis.get_euler()
	var yaw_rad: float = cam_euler.y
	var pitch_rad: float = cam_euler.x
	
	for inst in _layer_instances:
		if is_instance_valid(inst):
			inst.update_parallax(yaw_rad, pitch_rad)


func _validate_visual_only_hierarchy() -> void:
	for descendant in find_children("*", "", true, false):
		if descendant is CollisionObject3D or descendant is NavigationRegion3D:
			push_error("BackgroundController debe ser estrictamente visual: %s no esta permitido." % descendant.get_path())
