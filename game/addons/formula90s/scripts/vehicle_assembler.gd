class_name VehicleAssembler
extends Node

## VehicleAssembler dynamically configures a Vehicle (RigidBody3D) from a VehicleSpec resource.
## This decouples all physical tuning parameters from the Godot .tscn scene.

@export var spec: VehicleSpec
@export var vehicle_node: Vehicle

func _ready() -> void:
	if vehicle_node == null:
		vehicle_node = _resolve_vehicle()
	apply_spec()

func apply_spec() -> void:
	if vehicle_node == null:
		push_warning("VehicleAssembler: No Vehicle node found to apply spec.")
		return
	if spec == null:
		push_warning("VehicleAssembler: No VehicleSpec resource assigned.")
		return
	spec.apply_to(vehicle_node)
	if vehicle_node.is_node_ready():
		vehicle_node.axles.clear()
		vehicle_node.wheel_array.clear()
		vehicle_node.initialize()

func reload_spec(new_spec: VehicleSpec = null) -> void:
	if new_spec != null:
		spec = new_spec
	apply_spec()

func _resolve_vehicle() -> Vehicle:
	if get_parent() is Vehicle:
		return get_parent() as Vehicle
	var sibling := get_parent().get_node_or_null("VehicleRigidBody") as Vehicle
	if sibling != null:
		return sibling
	return null
