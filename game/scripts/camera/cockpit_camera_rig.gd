extends Node3D

const COCKPIT_CONFIGURATION_SCRIPT := preload("res://scripts/camera/cockpit_camera_configuration.gd")

@export var driver_controller_path: NodePath
@export_file("*.json") var configuration_path: String

var driver_controller: Node
var driver_eye_point: Node3D
var camera: Camera3D
var configuration: CockpitCameraConfiguration

func _ready() -> void:
	top_level = true
	driver_controller = get_node_or_null(driver_controller_path)
	camera = get_node_or_null("Camera3D") as Camera3D
	configuration = COCKPIT_CONFIGURATION_SCRIPT.load_from_path(configuration_path)
	if driver_controller == null or camera == null or configuration == null:
		push_error("Cockpit camera requires a driver controller, lens, and valid configuration.")
		return
	driver_eye_point = driver_controller.call("get_driver_eye_point") as Node3D
	if driver_eye_point == null:
		push_error("Cockpit camera requires the driver's authored eye point.")
		return
	camera.fov = configuration.field_of_view_degrees
	camera.near = configuration.near_clip_distance_meters
	camera.far = configuration.far_clip_distance_meters
	var skeleton := driver_controller.get("driver_skeleton") as Skeleton3D
	if skeleton != null:
		skeleton.skeleton_updated.connect(synchronize_camera, CONNECT_DEFERRED)
	synchronize_camera()

func synchronize_camera() -> void:
	if is_instance_valid(driver_eye_point):
		global_transform = driver_eye_point.global_transform.orthonormalized()
		global_position += global_basis.y * configuration.viewpoint_elevation_meters

func set_view_active(view_active: bool) -> void:
	if is_instance_valid(driver_controller):
		driver_controller.call("set_cockpit_view_active", view_active)
	if camera != null:
		camera.current = view_active
