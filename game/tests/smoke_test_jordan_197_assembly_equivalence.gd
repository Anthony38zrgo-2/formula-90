extends SceneTree

const SCENE := preload("res://scenes/tests/vehicle_track_combinations/jordan_197_assembly_smoke_test.tscn")
const MANIFEST_PATH := "res://assets/models/vehicles/jordan_197/vehicle_runtime_manifest.json"
const SOURCE_ASSEMBLY_PATH := "res://assets/models/vehicles/jordan_197/source_assembly.json"
const REPORT_PATH := "res://assets/models/vehicles/jordan_197/assembly_equivalence_report.json"
const SETTLE_FRAMES := 300
const RUNTIME_HUB_TOLERANCE_M := 0.002
const WHEELS := {
	"WheelFrontLeft": "DATUM_HUB_FL",
	"WheelFrontRight": "DATUM_HUB_FR",
	"WheelRearLeft": "DATUM_HUB_RL",
	"WheelRearRight": "DATUM_HUB_RR",
}


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var failures: Array[String] = []
	var manifest := _load_json(MANIFEST_PATH, failures)
	var source_assembly := _load_json(SOURCE_ASSEMBLY_PATH, failures)
	var report := _load_json(REPORT_PATH, failures)
	if report.get("status", "FAIL") != "PASS":
		failures.append("offline assembly equivalence report is not PASS")
	elif float(report.get("tolerance_mm", 0.0)) > 2.0:
		failures.append("offline assembly equivalence tolerance exceeds 2 mm")

	var scene_root := SCENE.instantiate()
	root.add_child(scene_root)
	var vehicle := scene_root.get_node_or_null("Jordan197/VehicleRigidBody") as RigidBody3D
	if vehicle == null:
		failures.append("canonical VehicleRigidBody is missing")
	else:
		for _frame in range(SETTLE_FRAMES):
			await physics_frame

		for wheel_name in WHEELS:
			var wheel := vehicle.get_node_or_null(wheel_name) as RayCast3D
			if wheel == null:
				failures.append("missing wheel %s" % wheel_name)
				continue
			var pivot: Node3D = wheel.get("wheel_node") as Node3D
			if pivot == null:
				failures.append("missing pivot for %s" % wheel_name)
				continue
			var expected := _source_anchor_runtime(
				source_assembly, manifest, WHEELS[wheel_name], failures
			)
			var actual := wheel.transform * pivot.position
			var error := expected.distance_to(actual)
			print(
				"ASSEMBLY_EQUIVALENCE_HUB %s expected=%s actual=%s error_mm=%.6f contact=%s"
				% [wheel_name, expected, actual, error * 1000.0, wheel.is_colliding()]
			)
			if error > RUNTIME_HUB_TOLERANCE_M:
				failures.append("%s hub error %.3f mm exceeds 2 mm" % [wheel_name, error * 1000.0])
			if not wheel.is_colliding():
				failures.append("%s has no surface contact after settling" % wheel_name)

	scene_root.queue_free()
	await process_frame
	if failures.is_empty():
		print("PASS: Jordan 197 SOURCE/runtime/GEVP assembly equivalence")
		quit(0)
	else:
		push_error("; ".join(failures))
		quit(1)


func _load_json(path: String, failures: Array[String]) -> Dictionary:
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		failures.append("cannot open %s" % path)
		return {}
	var parsed: Variant = JSON.parse_string(file.get_as_text())
	if not parsed is Dictionary:
		failures.append("invalid JSON: %s" % path)
		return {}
	return parsed as Dictionary


func _source_anchor_runtime(
	source_assembly: Dictionary,
	manifest: Dictionary,
	anchor_name: String,
	failures: Array[String]
) -> Vector3:
	var anchors: Dictionary = source_assembly.get("anchors_source_m", {})
	var source_point: Array = anchors.get(anchor_name, [])
	var runtime: Dictionary = manifest.get("runtime", {})
	var rotation: Array = runtime.get("source_to_runtime_rotation_3x3", [])
	var translation: Array = runtime.get("translation_after_rotation", [])
	if source_point.size() != 3 or rotation.size() != 3 or translation.size() != 3:
		failures.append("cannot transform SOURCE anchor %s" % anchor_name)
		return Vector3.ZERO
	var point := Vector3(float(source_point[0]), float(source_point[1]), float(source_point[2]))
	return Vector3(
		float(rotation[0][0]) * point.x + float(rotation[0][1]) * point.y + float(rotation[0][2]) * point.z + float(translation[0]),
		float(rotation[1][0]) * point.x + float(rotation[1][1]) * point.y + float(rotation[1][2]) * point.z + float(translation[1]),
		float(rotation[2][0]) * point.x + float(rotation[2][1]) * point.y + float(rotation[2][2]) * point.z + float(translation[2])
	)
