extends Control

const WORLD_VIEWPORT_PATH := NodePath("WorldViewport")
const HUD_LAYER_PATH := NodePath("HudLayer")
const WORLD_CONTENT_NAME := &"WorldContent"

const Resolver := preload("res://addons/formula90s/scripts/vehicle_path_resolver.gd")

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
	_bind_handling_tuner(world_content)


func _resolve_world_scene_path() -> String:
	if not world_scene_path.is_empty():
		return world_scene_path

	var bootstrap := get_parent()
	if bootstrap != null and bootstrap.has_method("get_track_scene_path"):
		return String(bootstrap.call("get_track_scene_path"))
	return ""


func _find_vehicle(world_content: Node) -> Node3D:
	return Resolver.find_canonical(world_content) as Node3D

func _vehicle_path_string(vehicle: Node3D) -> String:
	if vehicle == null:
		return String(Resolver.CANDIDATE_PATHS[0])
	var world_content := vehicle
	while world_content != null and world_content.name != WORLD_CONTENT_NAME:
		world_content = world_content.get_parent()
	if world_content != null:
		var rel := world_content.get_path_to(vehicle)
		return String(rel)
	# fallback
	var parent := vehicle.get_parent()
	if parent != null:
		return String(parent.name) + "/VehicleRigidBody"
	return String(Resolver.CANDIDATE_PATHS[0])

func _extract_hud(world_content: Node) -> void:
	var vehicle := _find_vehicle(world_content)
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

	var vehicle_suffix := _vehicle_path_string(vehicle)
	var world_vehicle_path := NodePath("../../WorldViewport/WorldContent/" + vehicle_suffix)
	var world_vehicle_path_deep := NodePath("../../../WorldViewport/WorldContent/" + vehicle_suffix)

	if control.name == &"DebugHud":
		control.set("vehicle_path", world_vehicle_path)
		if has_aids:
			control.set("aids_path", NodePath("../../WorldViewport/WorldContent/DrivingAids"))

		var minimap := control.get_node_or_null("Minimap")
		if minimap != null:
			minimap.set("target_path", world_vehicle_path_deep)
			if minimap.has_method("set_target"):
				minimap.call("set_target", vehicle)
	elif control.name == &"WheelDiagnostics":
		control.set("vehicle_path", world_vehicle_path)


func _clear_runtime_owners(node: Node) -> void:
	node.owner = null
	for child in node.get_children():
		_clear_runtime_owners(child)


func _bind_handling_tuner(world_content: Node) -> void:
	var vehicle := _find_vehicle(world_content) as Vehicle
	var tuner := hud_layer.get_node_or_null("DebugHud/HandlingTuningPanel")
	if vehicle != null and tuner != null and tuner.has_method("bind_vehicle"):
		tuner.call("bind_vehicle", vehicle, vehicle.get_parent())
