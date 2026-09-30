class_name SteeringWheelVisualController
extends Node

@export var vehicle: Node3D
@export var chassis_visual: Node3D
@export_range(1.0, 1440.0, 1.0) var total_rotation_degrees: float = 360.0

var steering_wheel_pivot: Node3D

func _ready() -> void:
	if vehicle == null or chassis_visual == null or not vehicle.has_method("get_true_steering_amount"):
		push_error("Steering wheel animation requires a chassis and effective steering telemetry.")
		set_process(false)
		return
	var steering_wheel := chassis_visual.find_child("GEO_CHASSIS_STEER", true, false) as MeshInstance3D
	var steering_column := chassis_visual.find_child("GEO_CHASSIS_STEERCOLUM", true, false) as MeshInstance3D
	if steering_wheel == null or steering_column == null:
		push_error("Steering wheel animation requires the steering wheel and column meshes.")
		set_process(false)
		return
	var chassis_inverse := chassis_visual.global_transform.affine_inverse()
	var steering_wheel_bounds: AABB = (chassis_inverse * steering_wheel.global_transform) * steering_wheel.get_aabb()
	var steering_column_bounds: AABB = (chassis_inverse * steering_column.global_transform) * steering_column.get_aabb()
	var steering_column_center := steering_column_bounds.get_center()
	steering_wheel_pivot = Node3D.new()
	steering_wheel_pivot.name = "SteeringWheelPivot"
	chassis_visual.add_child(steering_wheel_pivot)
	steering_wheel_pivot.position = Vector3(steering_column_center.x, steering_column_center.y, steering_wheel_bounds.get_center().z)
	steering_wheel.reparent(steering_wheel_pivot, true)
	update_steering_wheel_pose()

func _process(_elapsed_seconds: float) -> void:
	update_steering_wheel_pose()

func update_steering_wheel_pose() -> void:
	if steering_wheel_pivot == null:
		return
	var effective_steering_amount := float(vehicle.call("get_true_steering_amount"))
	if not is_finite(effective_steering_amount):
		effective_steering_amount = 0.0
	var steering_wheel_angle := clampf(effective_steering_amount, -1.0, 1.0) * deg_to_rad(total_rotation_degrees * 0.5)
	steering_wheel_pivot.basis = Basis(Vector3.BACK, steering_wheel_angle)
