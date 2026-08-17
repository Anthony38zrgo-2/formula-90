extends SceneTree

const SCENE_PATH := "res://scenes/vehicles/f1_94/f1_94_rust.tscn"

func _init() -> void:
	call_deferred("_run_test")

func _run_test() -> void:
	var failures: Array[String] = []
	print("=== F1-94 Differential Runtime Config Test ===")

	var packed = load(SCENE_PATH) as PackedScene
	if packed == null:
		printerr("[FAIL] Could not load f1_94_rust.tscn")
		quit(1)
		return

	var car = packed.instantiate()
	root.add_child(car)
	await process_frame
	await physics_frame
	print("[OK] scene instantiated")

	var vehicle = car.get_node_or_null("VehicleRigidBody")
	if vehicle == null:
		vehicle = car.find_child("VehicleRigidBody", true, false)
	if vehicle == null or not vehicle.has_method("get_diff_preload"):
		printerr("[FAIL] VehicleRigidBody missing or no diff API (car=%s vehicle=%s)" % [car, vehicle])
		quit(1)
		return

	# Defaults are loaded from the authoritative JSON (data/vehicles/f1_94/f1_94_physics.json).
	# The JSON differential section sets preload_nm=40.0, clutch_friction_coefficient=0.1.
	var dp = vehicle.get_diff_preload()
	var mu = vehicle.get_diff_clutch_friction_coeff()
	print("[OK] default diff_preload=%.3f (expect 40) mu=%.3f (expect 0.1)" % [dp, mu])
	if abs(dp - 40.0) > 1e-6:
		failures.append("default diff_preload=%f expected 40 (JSON)" % dp)
	if abs(mu - 0.1) > 1e-6:
		failures.append("default mu=%f expected 0.1 (JSON)" % mu)

	# GEVP alias: rear_locking_differential_engage_torque=170 -> preload=170, mu=0
	vehicle.set_rear_locking_differential_engage_torque(170.0)
	await physics_frame
	var dp2 = vehicle.get_diff_preload()
	var mu2 = vehicle.get_diff_clutch_friction_coeff()
	print("[OK] after engage_torque=170 -> diff_preload=%.3f mu=%.3f" % [dp2, mu2])
	if abs(dp2 - 170.0) > 1e-6:
		failures.append("engage_torque mapping diff_preload=%f expected 170" % dp2)
	if abs(mu2 - 0.0) > 1e-6:
		failures.append("engage_torque mapping mu=%f expected 0.0" % mu2)

	# Direct Salisbury tuning survives a separate setter without clobbering
	vehicle.set_diff_preload(200.0)
	vehicle.set_diff_clutch_friction_coeff(0.10)
	await physics_frame
	var dp3 = vehicle.get_diff_preload()
	var mu3 = vehicle.get_diff_clutch_friction_coeff()
	print("[OK] direct tuning -> diff_preload=%.3f mu=%.3f" % [dp3, mu3])
	if abs(dp3 - 200.0) > 1e-6:
		failures.append("direct diff_preload=%f expected 200" % dp3)
	if abs(mu3 - 0.10) > 1e-6:
		failures.append("direct mu=%f expected 0.10" % mu3)

	if failures.size() == 0:
		print("=== F1-94 Differential Runtime Config Test: PASSED ===")
		quit(0)
	else:
		for f in failures:
			printerr("[FAIL] " + f)
		printerr("=== F1-94 Differential Runtime Config Test: FAILED (%d) ===" % failures.size())
		quit(1)
