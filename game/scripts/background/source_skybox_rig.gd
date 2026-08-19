extends Node3D

## Keeps the visual-only background centered on the active camera without
## inheriting its rotation. This gives the distant cards the stable, cubemap-like
## behavior of a Source-style 3D skybox while preserving normal depth testing.

var _camera: Camera3D


func _ready() -> void:
	_validate_visual_only_hierarchy()
	_resolve_current_camera()
	_follow_active_camera()


func _process(_delta: float) -> void:
	_resolve_current_camera()
	_follow_active_camera()


func _resolve_current_camera() -> void:
	var current_camera := get_viewport().get_camera_3d()
	if current_camera != null:
		_camera = current_camera


func _follow_active_camera() -> void:
	if not is_instance_valid(_camera):
		return
	global_position = Vector3(_camera.global_position.x, global_position.y, _camera.global_position.z)


func _validate_visual_only_hierarchy() -> void:
	for descendant in find_children("*", "", true, false):
		if descendant is CollisionObject3D or descendant is NavigationRegion3D:
			push_error("SourceSkyboxRig must remain visual-only: %s is not allowed." % descendant.get_path())
