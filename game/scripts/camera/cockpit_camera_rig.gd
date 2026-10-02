extends Node3D

const COCKPIT_PREFERENCES_PANEL_SCRIPT := preload("res://scripts/camera/cockpit_camera_preferences_panel.gd")

@export var driver_controller_path: NodePath
@export_file("*.json") var configuration_path: String
@export var load_saved_preferences := true

var driver_controller: Node
var driver_eye_point: Node3D
var camera: Camera3D
var configuration: CockpitCameraConfiguration
var vehicle: Node3D
var head_motion_modifier: SkeletonModifier3D
var motion_state: RefCounted
var positional_correction_world := Vector3.ZERO
var preferences_panel: CanvasLayer

func _ready() -> void:
	top_level = true
	physics_interpolation_mode = Node.PHYSICS_INTERPOLATION_MODE_OFF
	driver_controller = get_node_or_null(driver_controller_path)
	camera = get_node_or_null("Camera3D") as Camera3D
	if driver_controller == null or camera == null:
		push_error("Cockpit camera requires a driver controller and lens.")
		return
	driver_eye_point = driver_controller.call("get_driver_eye_point") as Node3D
	head_motion_modifier = driver_controller.get("head_motion_modifier") as SkeletonModifier3D
	if driver_eye_point == null or head_motion_modifier == null:
		push_error("Cockpit camera requires the driver's authored eye point and shared motion.")
		return
	configuration = head_motion_modifier.get("configuration") as CockpitCameraConfiguration
	motion_state = head_motion_modifier.get("motion_state") as RefCounted
	vehicle = head_motion_modifier.get("vehicle") as Node3D
	if configuration == null or motion_state == null:
		push_error("Cockpit camera requires a valid shared motion configuration.")
		return
	camera.fov = configuration.field_of_view_degrees
	camera.near = configuration.near_clip_distance_meters
	camera.far = configuration.far_clip_distance_meters
	preferences_panel = COCKPIT_PREFERENCES_PANEL_SCRIPT.new() as CanvasLayer
	preferences_panel.set("configuration", configuration)
	preferences_panel.set("load_saved_preferences", load_saved_preferences)
	preferences_panel.set("camera_rig", self)
	get_tree().root.add_child.call_deferred(preferences_panel)
	var skeleton := driver_controller.get("driver_skeleton") as Skeleton3D
	if skeleton != null:
		skeleton.skeleton_updated.connect(synchronize_camera)
	synchronize_camera()

func _exit_tree() -> void:
	if is_instance_valid(preferences_panel):
		preferences_panel.queue_free()

func _process(_elapsed_seconds: float) -> void:
	synchronize_camera()

func _unhandled_input(input_event: InputEvent) -> void:
	if camera != null and camera.current and input_event is InputEventKey and input_event.pressed and not input_event.echo and input_event.physical_keycode == KEY_F9:
		preferences_panel.visible = not preferences_panel.visible
		get_viewport().set_input_as_handled()

func get_rendered_eye_transform() -> Transform3D:
	return driver_eye_point.global_transform.orthonormalized()

func get_viewpoint_elevation_direction() -> Vector3:
	var rendered_state: Dictionary = motion_state.call("get_render_state", Engine.get_physics_interpolation_fraction())
	var heading_basis: Basis = motion_state.call("make_heading_basis", vehicle.global_basis)
	var reference_angles: Vector2 = rendered_state["road_reference_angles"]
	return (heading_basis * Basis.from_euler(Vector3(reference_angles.x, 0.0, reference_angles.y))).y

func synchronize_camera() -> void:
	if not is_instance_valid(driver_eye_point) or motion_state == null:
		return
	var rendered_state: Dictionary = motion_state.call("get_render_state", Engine.get_physics_interpolation_fraction())
	var eye_transform := get_rendered_eye_transform()
	positional_correction_world = rendered_state["positional_correction_world"]
	var lateral_direction := Vector3(vehicle.global_basis.x.x, 0.0, vehicle.global_basis.x.z).normalized()
	var lateral_correction := float(motion_state.call("soft_limit", positional_correction_world.dot(lateral_direction), configuration.maximum_lateral_correction_meters))
	var vertical_correction := float(motion_state.call("soft_limit", positional_correction_world.y + float(rendered_state["vertical_displacement_meters"]), configuration.maximum_vertical_correction_meters))
	positional_correction_world = lateral_direction * lateral_correction + Vector3.UP * vertical_correction
	global_transform = eye_transform
	global_position = eye_transform.origin + get_viewpoint_elevation_direction() * configuration.viewpoint_elevation_meters + positional_correction_world * configuration.positional_stabilization_strength

func reset_positional_motion() -> void:
	if motion_state != null:
		motion_state.call("reset_positional_motion")
	positional_correction_world = Vector3.ZERO

func set_view_active(view_active: bool) -> void:
	if view_active:
		reset_positional_motion()
	elif preferences_panel != null:
		preferences_panel.visible = false
	if is_instance_valid(driver_controller):
		driver_controller.call("set_cockpit_view_active", view_active)
	if camera != null:
		camera.current = view_active
