extends SceneTree

const DEFAULT_SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"

func _init() -> void:
	call_deferred("_run")


func _worst_remaining_fraction(snapshot: Dictionary) -> float:
	var worst := 2.0
	var found := false
	for wheel in ["FL", "FR", "RL", "RR"]:
		var wheel_value: Variant = snapshot.get(wheel, {})
		if not (wheel_value is Dictionary):
			continue
		var remaining: Variant = wheel_value.get("wear_remaining_fraction")
		if remaining == null:
			continue
		found = true
		worst = minf(worst, float(remaining))
	return worst if found else -1.0


func _run() -> void:
	var failures: Array[String] = []
	var scene_path := DEFAULT_SCENE_PATH
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--scene="):
			scene_path = argument.trim_prefix("--scene=")
	var packed := load(scene_path) as PackedScene
	if packed == null:
		printerr("[FAIL] tire wear runtime scene could not load: " + scene_path)
		quit(1)
		return
	var compositor := packed.instantiate()
	root.add_child(compositor)
	for _frame in 8:
		await process_frame

	var vehicle := compositor.find_child("VehicleRigidBody", true, false)
	if vehicle == null:
		failures.append("VehicleRigidBody is missing")
	elif not vehicle.has_method("get_tire_state_snapshot"):
		failures.append("vehicle does not expose get_tire_state_snapshot")
	else:
		if "enable_player_input" in vehicle:
			vehicle.set("enable_player_input", false)
		var before: Dictionary = vehicle.call("get_tire_state_snapshot")
		var before_remaining := _worst_remaining_fraction(before)
		if before_remaining < 0.0:
			failures.append("wear snapshot keys are missing before driving: %s" % [before])
		else:
			for frame in 2400:
				vehicle.set("throttle_amount", 0.6)
				vehicle.set("steering_input", 0.03 * sin(float(frame) * 0.012))
				vehicle.set("brake_amount", 0.4 if frame % 360 > 350 else 0.0)
				await physics_frame
			var after: Dictionary = vehicle.call("get_tire_state_snapshot")
			var after_remaining := _worst_remaining_fraction(after)
			print("[WEAR] remaining before=%.5f after=%.5f" % [before_remaining, after_remaining])
			print("[WEAR] front_left=%s" % [after.get("FL", {})])
			if after_remaining < 0.0:
				failures.append("wear snapshot keys are missing after driving: %s" % [after])
			elif after_remaining >= before_remaining:
				failures.append("tire wear did not progress during the runtime stint: before=%.5f after=%.5f" % [before_remaining, after_remaining])
			var grip_value: Variant = after.get("FL", {}).get("wear_grip_scale") if after.get("FL", {}) is Dictionary else null
			if grip_value == null or float(grip_value) > 1.0 or float(grip_value) <= 0.0:
				failures.append("FL wear_grip_scale is invalid: %s" % [grip_value])

	compositor.queue_free()
	if failures.is_empty():
		print("[PASS] Tire wear state is exposed and progresses during the canonical runtime stint.")
	else:
		for failure in failures:
			printerr("[FAIL] " + failure)
	quit(failures.size())
