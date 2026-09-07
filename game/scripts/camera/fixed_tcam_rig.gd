class_name FixedTCamRig
extends Node3D

## Fixed T-cam: rigid onboard camera anchored to the vehicle.
## No smoothing, no inertia, no FOV effects. Copies the vehicle transform
## every physics frame with a constant local offset (x=0 centered) and a
## constant downward pitch so the nose + track + horizon stay framed.

const TCamConfigScript := preload("res://scripts/camera/tcam_config.gd")

@export var car_path: NodePath = NodePath("../VehicleContainer/ActiveVehicle/VehicleRigidBody")
@export var config_path: String = "res://data/cameras/fixed_tcam.json"

var config
var _car: Node3D


func _ready() -> void:
	top_level = true
	config = TCamConfigScript.load_from_json(config_path)
	_car = get_node_or_null(car_path) as Node3D
	if _car == null:
		push_warning("FixedTCamRig: car not found at '%s'." % car_path)
		return
	_apply_lens()
	_snap_rigid()


func _physics_process(_delta: float) -> void:
	if _car == null or not is_instance_valid(_car):
		return
	_snap_rigid()


func reload_config(path: String = "") -> void:
	if not path.is_empty():
		config_path = path
	config = TCamConfigScript.load_from_json(config_path)
	_apply_lens()
	_snap_rigid()


func get_camera() -> Camera3D:
	return get_node_or_null("Camera3D") as Camera3D


func _apply_lens() -> void:
	var cam := get_camera()
	if cam == null or config == null:
		return
	cam.fov = config.fov_deg
	cam.near = config.near_m
	cam.far = config.far_m


func _snap_rigid() -> void:
	if _car == null or config == null:
		return
	var tilt := Basis(Vector3.RIGHT, config.pitch_rad())
	var local := Transform3D(tilt, config.local_offset())
	global_transform = _car.global_transform * local
