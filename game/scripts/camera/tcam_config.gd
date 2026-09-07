class_name TCamConfig
extends RefCounted

## Data-driven fixed T-cam configuration.
## Loads game/data/cameras/fixed_tcam.json and exposes rigid-mount knobs
## (offsets in vehicle space, pitch, fov). No smoothing or motion effects here.

const DEFAULT_PATH := "res://data/cameras/fixed_tcam.json"

# Local offset in vehicle space: x = lateral (0 = centered), y = height above
# VehicleRigidBody origin, z = longitudinal (+ = rear, T-cam sits behind head).
var height_offset_m := 0.70
var lateral_offset_m := 0.0
var longitudinal_offset_m := 0.30
var pitch_deg := -5.0
var fov_deg := 65.0
var near_m := 0.05
var far_m := 2000.0


static func load_from_json(path: String = DEFAULT_PATH) -> RefCounted:
	var config = (load("res://scripts/camera/tcam_config.gd") as GDScript).new()
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		push_warning("T-cam config missing: %s. Using defaults." % path)
		return config
	var parsed: Variant = JSON.parse_string(file.get_as_text())
	if not (parsed is Dictionary):
		push_warning("T-cam config is invalid: %s. Using defaults." % path)
		return config
	config.apply(parsed)
	return config


func apply(data: Dictionary) -> void:
	height_offset_m = clampf(float(data.get("height_offset_m", height_offset_m)), 0.5, 1.2)
	lateral_offset_m = clampf(float(data.get("lateral_offset_m", lateral_offset_m)), -0.5, 0.5)
	longitudinal_offset_m = clampf(float(data.get("longitudinal_offset_m", longitudinal_offset_m)), -1.0, 1.5)
	pitch_deg = clampf(float(data.get("pitch_deg", pitch_deg)), -15.0, 0.0)
	fov_deg = clampf(float(data.get("fov_deg", fov_deg)), 40.0, 90.0)
	near_m = clampf(float(data.get("near_m", near_m)), 0.01, 1.0)
	far_m = clampf(float(data.get("far_m", far_m)), 100.0, 8000.0)


func local_offset() -> Vector3:
	return Vector3(lateral_offset_m, height_offset_m, longitudinal_offset_m)


func pitch_rad() -> float:
	return deg_to_rad(pitch_deg)
