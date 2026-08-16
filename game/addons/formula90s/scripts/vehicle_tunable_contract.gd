class_name VehicleTunableContract
extends RefCounted

## VehicleTunableContract provides typed and bounded read/write access to Vehicle runtime properties.
## Replaces untyped reflection and protects against out-of-bounds physical instability.

const PROPERTY_LIMITS := {
	"front_brake_bias": {"min": 0.30, "max": 0.85},
	"max_steering_angle": {"min": 0.05, "max": 1.0},
	"max_torque": {"min": 50.0, "max": 2000.0},
	"motor_drag": {"min": 0.0, "max": 0.1},
	"coefficient_of_drag": {"min": 0.05, "max": 2.5},
	"frontal_area": {"min": 0.2, "max": 3.0},
	"stability_yaw_strength": {"min": 0.0, "max": 30.0},
	"steering_exponent": {"min": 0.5, "max": 4.0},
	"brake_force_multiplier": {"min": 0.1, "max": 5.0}
}

static func get_value(vehicle: Node, property_name: String) -> Variant:
	if vehicle == null:
		return null
	return vehicle.get(property_name)

static func set_value(vehicle: Node, property_name: String, value: Variant) -> bool:
	if vehicle == null:
		return false
	
	var final_value = value
	if PROPERTY_LIMITS.has(property_name) and (value is float or value is int):
		var lim = PROPERTY_LIMITS[property_name]
		final_value = clampf(float(value), lim["min"], lim["max"])
	
	vehicle.set(property_name, final_value)
	return true
