extends Control

const WORLD_VIEWPORT_PATH := NodePath("WorldViewport")
const HUD_LAYER_PATH := NodePath("HudLayer")
const WORLD_CONTENT_NAME := &"WorldContent"
const VEHICLE_PATH := NodePath("VehicleController/VehicleRigidBody")

@export_file("*.tscn") var world_scene_path := ""

@onready var world_viewport: SubViewport = get_node(WORLD_VIEWPORT_PATH)
@onready var world_presenter: TextureRect = $WorldPresenter
@onready var hud_layer: CanvasLayer = get_node(HUD_LAYER_PATH)


func _ready() -> void:
	world_presenter.texture = world_viewport.get_texture()
	_load_world()


func _load_world() -> void:
	var scene_path := _resolve_world_scene_path()
	if scene_path.is_empty():
		push_error("WorldHudCompositor could not resolve a world scene path.")
		return

	var world_scene := load(scene_path) as PackedScene
	if world_scene == null:
		push_error("WorldHudCompositor could not load %s." % scene_path)
		return

	var world_content := world_scene.instantiate()
	world_content.name = WORLD_CONTENT_NAME
	_extract_hud(world_content)
	world_viewport.add_child(world_content)


func _resolve_world_scene_path() -> String:
	if not world_scene_path.is_empty():
		return world_scene_path

	var bootstrap := get_parent()
	if bootstrap != null and bootstrap.has_method("get_track_scene_path"):
		return String(bootstrap.call("get_track_scene_path"))
	return ""


func _extract_hud(world_content: Node) -> void:
	var vehicle := world_content.get_node_or_null(VEHICLE_PATH) as Node3D
	var has_aids := world_content.get_node_or_null("DrivingAids") != null
	var root_controls: Array[Control] = []
	for child in world_content.get_children():
		if child is Control:
			root_controls.append(child)

	for control in root_controls:
		world_content.remove_child(control)
		_clear_runtime_owners(control)
		_retarget_hud(control, vehicle, has_aids)
		hud_layer.add_child(control)


func _retarget_hud(control: Control, vehicle: Node3D, has_aids: bool) -> void:
	if vehicle == null:
		return

	if control.name == &"DebugHud":
		control.set("vehicle_path", NodePath("../../WorldViewport/WorldContent/VehicleController/VehicleRigidBody"))
		if has_aids:
			control.set("aids_path", NodePath("../../WorldViewport/WorldContent/DrivingAids"))

		var minimap := control.get_node_or_null("Minimap")
		if minimap != null:
			minimap.set("target_path", NodePath("../../../WorldViewport/WorldContent/VehicleController/VehicleRigidBody"))
			if minimap.has_method("set_target"):
				minimap.call("set_target", vehicle)
	elif control.name == &"WheelDiagnostics":
		control.set("vehicle_path", NodePath("../../WorldViewport/WorldContent/VehicleController/VehicleRigidBody"))


func _clear_runtime_owners(node: Node) -> void:
	node.owner = null
	for child in node.get_children():
		_clear_runtime_owners(child)
