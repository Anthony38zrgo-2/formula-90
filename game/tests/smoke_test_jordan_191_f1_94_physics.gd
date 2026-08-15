extends SceneTree

const SCENE_PATH := "res://scenes/tests/vehicle_track_combinations/jordan_191_f1_94_physics_test.tscn"
const EXPECTED_VISUALS := {
	"WheelFrontLeft": "res://scenes/vehicles/jordan_191/jordan_191_wheel_fl.glb",
	"WheelFrontRight": "res://scenes/vehicles/jordan_191/jordan_191_wheel_fr.glb",
	"WheelRearLeft": "res://scenes/vehicles/jordan_191/jordan_191_wheel_rl.glb",
	"WheelRearRight": "res://scenes/vehicles/jordan_191/jordan_191_wheel_rr.glb",
}
const F194_PHYSICS := {
	"vehicle_mass": 505.0,
	"max_torque": 340.0,
	"max_rpm": 17000.0,
	"front_tire_radius": 0.316954494,
	"rear_tire_radius": 0.329008996,
	"rear_spring_length": 0.18,
	"coefficient_of_drag": 0.15,
	"frontal_area": 0.45,
}


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var failures: Array[String] = []
	var vehicle_scene_text := FileAccess.get_file_as_string("res://scenes/vehicles/jordan_191/jordan_191_f1_94_physics.tscn")
	for wheel_name in EXPECTED_VISUALS:
		if not vehicle_scene_text.contains(EXPECTED_VISUALS[wheel_name]):
			failures.append("Jordan 191 asset is not declared in the isolated scene: %s" % wheel_name)
	var packed := load(SCENE_PATH) as PackedScene
	if packed == null:
		printerr("[FAIL] Alternate Jordan 191 F1-94 physics scene could not load.")
		quit(1)
		return
	var test_scene := packed.instantiate()
	root.add_child(test_scene)
	for _frame in 4:
		await process_frame
	var vehicle := test_scene.get_node_or_null("Jordan191F194Physics/VehicleRigidBody") as RigidBody3D
	if vehicle == null:
		failures.append("VehicleRigidBody is missing.")
	else:
		for property_name in F194_PHYSICS:
			if not is_equal_approx(float(vehicle.get(property_name)), F194_PHYSICS[property_name]):
				failures.append("F1-94 physics mismatch: %s" % property_name)
		for wheel_name in EXPECTED_VISUALS:
			var wheel := vehicle.get_node_or_null(wheel_name) as RayCast3D
			var visual := wheel.get_node_or_null(str(wheel.wheel_node) + "/Visual") if wheel != null else null
			if wheel == null or visual == null or visual.get_child_count() == 0:
				failures.append("Jordan 191 visual did not instantiate: %s" % wheel_name)
	for _frame in 180:
		await physics_frame
	if vehicle != null:
		var contacts := 0
		for wheel_name in EXPECTED_VISUALS:
			var wheel := vehicle.get_node_or_null(wheel_name) as RayCast3D
			if wheel != null and wheel.is_colliding():
				contacts += 1
		if contacts != 4:
			failures.append("Expected 4/4 wheel contacts after settling; got %d." % contacts)
	test_scene.queue_free()
	if failures.is_empty():
		print("[PASS] Jordan 191 alternate route uses the F1-94 physics baseline with all five Jordan visuals.")
	else:
		for failure in failures:
			printerr("[FAIL] " + failure)
	quit(failures.size())
