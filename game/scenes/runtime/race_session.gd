class_name RaceSession
extends Node3D

signal composition_ready(vehicle: Node, track: Node3D, aids: DrivingAidsController)

const CAMERA_SCENE := preload("res://scenes/runtime/arcade_chase_camera_rig.tscn")
const TCAM_SCENE := preload("res://scenes/runtime/fixed_tcam_rig.tscn")
const AIDS_SCRIPT := preload("res://scripts/vehicle/driving_aids.gd")

@export var config: RaceSessionConfig

var active_track: Node3D
var active_vehicle_root: Node3D
var active_vehicle: Node
var driving_aids: DrivingAidsController
var background_controller: BackgroundController
var background_skybox: BackgroundSkybox
# Deprecated compatibility handle. Factory-authored tracks no longer
# instantiate BackgroundMountains3D at runtime.
var background_mountains_3d: BackgroundMountains3D
var camera_rig_chase: Node3D
var camera_rig_tcam: Node3D
var use_tcam := false

func _ready() -> void:
	if config != null:
		compose(config)

func compose(next_config: RaceSessionConfig) -> bool:
	if next_config == null or not next_config.is_valid_config():
		push_error("RaceSession requires valid vehicle and track definitions.")
		return false
	_clear_composition()
	config = next_config
	active_track = config.selected_track.track_scene.instantiate() as Node3D
	active_track.name = "ActiveTrack"
	$TrackContainer.add_child(active_track)
	var spawn := _find_vehicle_spawn(active_track)
	if spawn == null:
		push_error("Track '%s' has no VehicleSpawn Marker3D." % config.selected_track.id)
		_clear_composition()
		return false
	active_vehicle_root = config.selected_vehicle.vehicle_scene.instantiate() as Node3D
	active_vehicle_root.name = "ActiveVehicle"
	$VehicleContainer.add_child(active_vehicle_root)
	active_vehicle_root.global_transform = Transform3D.IDENTITY
	active_vehicle = active_vehicle_root.get_node_or_null("VehicleRigidBody")
	if active_vehicle == null:
		push_error("Vehicle '%s' does not expose VehicleRigidBody." % config.selected_vehicle.id)
		_clear_composition()
		return false
	if active_vehicle.has_method("reset_vehicle"):
		var spawn_height: float = 0.35
		if active_vehicle.has_method("get_default_spawn_height"):
			spawn_height = float(active_vehicle.get_default_spawn_height())
		elif "default_spawn_height" in active_vehicle:
			spawn_height = float(active_vehicle.default_spawn_height)
		var spawn_pos := spawn.global_position
		spawn_pos.y += spawn_height
		active_vehicle.reset_vehicle(spawn_pos, spawn.global_rotation.y)
	_add_runtime_systems()
	composition_ready.emit(active_vehicle, active_track, driving_aids)
	return true

func _find_vehicle_spawn(track: Node) -> Marker3D:
	var named := track.find_child("VehicleSpawn", true, false) as Marker3D
	if named != null:
		return named
	for candidate in get_tree().get_nodes_in_group("vehicle_spawn"):
		if candidate is Marker3D and track.is_ancestor_of(candidate):
			return candidate as Marker3D
	return null

func _add_runtime_systems() -> void:
	camera_rig_chase = CAMERA_SCENE.instantiate() as Node3D
	camera_rig_chase.name = "CameraRig"
	camera_rig_chase.set("car_path", NodePath("../VehicleContainer/ActiveVehicle/VehicleRigidBody"))
	add_child(camera_rig_chase)

	camera_rig_tcam = TCAM_SCENE.instantiate() as Node3D
	camera_rig_tcam.name = "CameraRigTCam"
	camera_rig_tcam.set("car_path", NodePath("../VehicleContainer/ActiveVehicle/VehicleRigidBody"))
	add_child(camera_rig_tcam)
	_apply_active_camera(false)

	driving_aids = AIDS_SCRIPT.new() as DrivingAidsController
	driving_aids.name = "DrivingAids"
	driving_aids.vehicle_node = active_vehicle
	add_child(driving_aids)

	_setup_background(camera_rig_chase)


func _process(_delta: float) -> void:
	# Polling global en vez de _unhandled_input: RaceSession vive dentro del
	# SubViewport (WorldViewport) y los eventos de tecla no siempre se propagan
	# a _unhandled_input de nodos embebidos. Input.is_action_just_pressed es
	# independiente del viewport/foco (mismo patron que VehicleRustInputController).
	if InputMap.has_action("Toggle Camera") and Input.is_action_just_pressed("Toggle Camera"):
		toggle_camera()


func toggle_camera() -> void:
	_apply_active_camera(not use_tcam)


func is_tcam_active() -> bool:
	return use_tcam


func _apply_active_camera(enable_tcam: bool) -> void:
	use_tcam = enable_tcam
	var chase_cam := _find_camera3d_recursive(camera_rig_chase) if camera_rig_chase != null else null
	var tcam_cam := _find_camera3d_recursive(camera_rig_tcam) if camera_rig_tcam != null else null
	if chase_cam != null:
		chase_cam.current = not use_tcam
	if tcam_cam != null:
		tcam_cam.current = use_tcam
	_rebind_background_to_active_camera()

func _find_camera3d_recursive(node: Node) -> Camera3D:
	if node is Camera3D:
		return node as Camera3D
	for child in node.get_children():
		var found := _find_camera3d_recursive(child)
		if found != null:
			return found
	return null


func _setup_background(camera_rig: Node3D) -> void:
	if config == null or config.selected_track == null:
		return
	
	var cam := camera_rig.get_node_or_null("Camera3D") as Camera3D
	if cam == null:
		cam = _find_camera3d_recursive(camera_rig)
	if cam == null:
		push_warning("RaceSession: No se encontró Camera3D en camera_rig '%s'; skybox sin seguimiento." % camera_rig.name)
	
	# La Chutana's published factory GLB is the sole authority for Near/Far.
	# Keep its embedded materials and the track WorldEnvironment untouched.
	if config.selected_track.id == &"la_chutana":
		print("RaceSession: fondo de La Chutana provisto por el GLB de fábrica.")
		return
	
	# Fallback: legacy 2D parallax system
	var preset := config.selected_track.get_effective_background_preset()
	if preset == null:
		return
	
	# 1. Skybox desacoplado: sigue la camara y llena el frustum en todo momento.
	if preset.skybox != null and cam != null:
		var skybox := BackgroundSkybox.new()
		skybox.name = "BackgroundSkybox"
		if skybox.setup(preset.skybox):
			skybox.set_camera_source(cam)
			add_child(skybox)
			background_skybox = skybox
		else:
			push_warning("RaceSession: No se pudo activar el BackgroundSkybox; se conserva el fallback legacy.")
	
	# 2. Capas parallax gestionadas por BackgroundController.
	var controller := BackgroundController.new()
	controller.name = "BackgroundController"
	var success := controller.load_preset(preset)
	if success:
		if cam != null:
			controller.set_camera_source(cam)
		add_child(controller)
		background_controller = controller
		
		_hide_legacy_background()
	else:
		push_warning("RaceSession: No se pudo activar el BackgroundController; se conserva el fallback legacy.")


func _rebind_background_to_active_camera() -> void:
	var active_rig := camera_rig_tcam if use_tcam else camera_rig_chase
	if active_rig == null:
		return
	var cam := active_rig.get_node_or_null("Camera3D") as Camera3D
	if cam == null:
		cam = _find_camera3d_recursive(active_rig)
	if cam == null:
		return
	if background_skybox != null:
		background_skybox.set_camera_source(cam)
	if background_controller != null:
		background_controller.set_camera_source(cam)


func _hide_legacy_background() -> void:
	if active_track == null:
		return
	var legacy_skybox := active_track.get_node_or_null("SourceSkyboxRig") as Node3D
	if legacy_skybox != null:
		legacy_skybox.visible = false
		legacy_skybox.set_process(false)
	var world_env := active_track.get_node_or_null("WorldEnvironment") as WorldEnvironment
	if world_env != null and world_env.environment != null:
		world_env.environment.background_mode = Environment.BG_CANVAS
		world_env.environment.background_canvas_max_layer = 0

func _clear_composition() -> void:
	for container in [$TrackContainer, $VehicleContainer]:
		for child in container.get_children():
			container.remove_child(child)
			child.queue_free()
	for child_name in [&"CameraRig", &"CameraRigTCam", &"DrivingAids", &"BackgroundController", &"BackgroundSkybox", &"BackgroundMountains3D"]:
		var child := get_node_or_null(NodePath(String(child_name)))
		if child != null:
			remove_child(child)
			child.queue_free()
	active_track = null
	active_vehicle_root = null
	active_vehicle = null
	camera_rig_chase = null
	camera_rig_tcam = null
	use_tcam = false
	driving_aids = null
	background_controller = null
	background_skybox = null
	background_mountains_3d = null
