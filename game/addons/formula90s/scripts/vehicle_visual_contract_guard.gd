extends Node
class_name VehicleVisualContractGuard

## Read-only guard for the Formula90s canonical vehicle visual contract.
## It never changes wheel transforms or physics; it only reports scene regressions.

@export var vehicle_path := NodePath("..")
@export var position_tolerance := 0.0001
@export var orientation_tolerance := 0.001

func _ready() -> void:
	var vehicle := get_node_or_null(vehicle_path)
	if not vehicle:
		push_error("VehicleVisualContractGuard: vehicle not found at %s" % vehicle_path)
		return

	_validate_axle(vehicle, "front", "WheelFrontLeft", "WheelFrontRight")
	_validate_axle(vehicle, "rear", "WheelRearLeft", "WheelRearRight")

func _validate_axle(vehicle: Node, axle_name: String, left_name: String, right_name: String) -> void:
	var left := vehicle.get_node_or_null(left_name) as Node3D
	var right := vehicle.get_node_or_null(right_name) as Node3D
	if not left or not right:
		push_error("VehicleVisualContractGuard: %s axle wheel nodes missing" % axle_name)
		return

	if absf(left.position.x + right.position.x) > position_tolerance:
		push_error("VehicleVisualContractGuard: %s axle X positions are not mirrored: %.6f / %.6f" % [axle_name, left.position.x, right.position.x])
	if absf(left.position.y - right.position.y) > position_tolerance:
		push_error("VehicleVisualContractGuard: %s axle Y positions differ: %.6f / %.6f" % [axle_name, left.position.y, right.position.y])
	if absf(left.position.z - right.position.z) > position_tolerance:
		push_error("VehicleVisualContractGuard: %s axle Z positions differ: %.6f / %.6f" % [axle_name, left.position.z, right.position.z])

	var left_pivot := left.get_node_or_null("Pivot") as Node3D
	var right_pivot := right.get_node_or_null("Pivot") as Node3D
	if not left_pivot or not right_pivot:
		push_error("VehicleVisualContractGuard: %s axle requires Pivot below both RayCast3D wheels" % axle_name)
		return

	var left_orientation := left_pivot.get_node_or_null("Orientation") as Node3D
	var right_orientation := right_pivot.get_node_or_null("Orientation") as Node3D
	if not left_orientation or not right_orientation:
		push_error("VehicleVisualContractGuard: %s axle requires Orientation below both GEVP Pivots" % axle_name)
		return

	if absf(left_orientation.rotation.y) > orientation_tolerance:
		push_error("VehicleVisualContractGuard: %s left Orientation must remain identity" % axle_name)
	if absf(absf(right_orientation.rotation.y) - PI) > orientation_tolerance:
		push_error("VehicleVisualContractGuard: %s right Orientation must be 180 degrees around Y" % axle_name)

	var left_visual := left_orientation.get_node_or_null("Visual")
	var right_visual := right_orientation.get_node_or_null("Visual")
	if not left_visual or not right_visual:
		push_error("VehicleVisualContractGuard: %s axle visual nodes missing" % axle_name)
		return

	var left_scene_path := String(left_visual.scene_file_path)
	var right_scene_path := String(right_visual.scene_file_path)
	if not left_scene_path.is_empty() and not right_scene_path.is_empty() and left_scene_path != right_scene_path:
		push_error("VehicleVisualContractGuard: %s axle uses different wheel PackedScenes: %s / %s" % [axle_name, left_scene_path, right_scene_path])
