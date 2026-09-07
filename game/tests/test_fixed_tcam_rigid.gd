extends SceneTree

## Fixed T-cam rigidity suite: JSON loads, mount is centered/highest-point,
## pitch+FOV come from JSON, follow is rigid (no smoothing), toggle C exists.

const TCAM_SCRIPT := preload("res://scripts/camera/fixed_tcam_rig.gd")
const CONFIG_SCRIPT := preload("res://scripts/camera/tcam_config.gd")


func _init() -> void:
	call_deferred("_run")


func _fail(message: String, failures: Array[String]) -> void:
	printerr("[FAIL] " + message)
	failures.append(message)


func _run() -> void:
	var failures: Array[String] = []
	_test_config_defaults(failures)
	_test_config_file_values(failures)
	await _test_rigid_follow(failures)
	_test_toggle_action(failures)

	if failures.is_empty():
		print("[PASS] Fixed T-cam rigid suite passed.")
		quit(0)
	else:
		quit(failures.size())


func _test_config_defaults(failures: Array[String]) -> void:
	var cfg = CONFIG_SCRIPT.new()
	cfg.apply({})
	if absf(cfg.height_offset_m - 0.70) > 0.0001:
		_fail("TCamConfig default height changed: %f" % cfg.height_offset_m, failures)
		return
	if absf(cfg.lateral_offset_m) > 0.0001:
		_fail("TCamConfig must default laterally centered.", failures)
		return
	print("[OK] tcam defaults: centered, rigid.")


func _test_config_file_values(failures: Array[String]) -> void:
	var cfg = CONFIG_SCRIPT.load_from_json(CONFIG_SCRIPT.DEFAULT_PATH)
	if cfg == null:
		_fail("TCamConfig.load_from_json returned null.", failures)
		return
	if cfg.height_offset_m < 0.5 or cfg.height_offset_m > 1.2:
		_fail("TCam height_offset out of airbox range: %f" % cfg.height_offset_m, failures)
	if absf(cfg.lateral_offset_m) > 0.0001:
		_fail("TCam JSON must keep lateral_offset 0 (centered): %f" % cfg.lateral_offset_m, failures)
	if cfg.pitch_deg > 0.0 or cfg.pitch_deg < -15.0:
		_fail("TCam pitch must look down: %f" % cfg.pitch_deg, failures)
	if cfg.fov_deg < 40.0 or cfg.fov_deg > 90.0:
		_fail("TCam fov out of range: %f" % cfg.fov_deg, failures)
	if failures.is_empty():
		print("[OK] tcam JSON: h=%.2f pitch=%.1f fov=%.1f." % [cfg.height_offset_m, cfg.pitch_deg, cfg.fov_deg])


func _test_rigid_follow(failures: Array[String]) -> void:
	var holder := Node3D.new()
	holder.name = "TCamTestHolder"
	root.add_child(holder)

	var car := Node3D.new()
	car.name = "MockVehicle"
	holder.add_child(car)
	car.global_transform = Transform3D(Basis(), Vector3(10.0, 0.5, -20.0))

	var rig = TCAM_SCRIPT.new()
	rig.name = "CameraRigTCam"
	rig.car_path = NodePath("../MockVehicle")
	holder.add_child(rig)
	var cam := Camera3D.new()
	cam.name = "Camera3D"
	rig.add_child(cam)
	rig.reload_config()

	for _frame in 4:
		await physics_frame

	var cfg = rig.config
	var expected: Transform3D = car.global_transform * Transform3D(Basis(Vector3.RIGHT, cfg.pitch_rad()), cfg.local_offset())
	var pos_err: float = rig.global_position.distance_to(expected.origin)
	if pos_err > 0.001:
		_fail("TCam is not rigidly anchored: err=%.4f m." % pos_err, failures)
		holder.queue_free()
		return

	var local_in_car: Vector3 = car.global_transform.affine_inverse() * rig.global_position
	if absf(local_in_car.x) > 0.001:
		_fail("TCam is not horizontally centered: x=%.4f." % local_in_car.x, failures)
	if absf(local_in_car.y - cfg.height_offset_m) > 0.002:
		_fail("TCam height mismatch: y=%.4f expected %.2f." % [local_in_car.y, cfg.height_offset_m], failures)

	# Teleport: rigid mount must snap on the very next physics frame (no lag).
	car.global_position += Vector3(0.0, 10.0, 5.0)
	car.rotate_y(0.7)
	await physics_frame
	await physics_frame
	expected = car.global_transform * Transform3D(Basis(Vector3.RIGHT, cfg.pitch_rad()), cfg.local_offset())
	pos_err = rig.global_position.distance_to(expected.origin)
	if pos_err > 0.002:
		_fail("TCam did not snap rigidly after teleport: err=%.4f m." % pos_err, failures)
		holder.queue_free()
		return

	var fwd: Vector3 = -rig.global_transform.basis.z
	if fwd.y >= -0.01:
		_fail("TCam must pitch down toward track+horizon: fwd.y=%.4f." % fwd.y, failures)
	if absf(cam.fov - cfg.fov_deg) > 0.01:
		_fail("TCam lens must come from JSON: fov=%.2f expected %.2f." % [cam.fov, cfg.fov_deg], failures)

	holder.queue_free()
	if failures.is_empty():
		print("[OK] tcam rigid follow: centered, pitched down, snap-on-teleport.")


func _test_toggle_action(failures: Array[String]) -> void:
	if not InputMap.has_action("Toggle Camera"):
		_fail("InputMap is missing 'Toggle Camera' (C).", failures)
		return
	var found_c := false
	for ev in InputMap.action_get_events("Toggle Camera"):
		if ev is InputEventKey and (ev as InputEventKey).physical_keycode == KEY_C:
			found_c = true
	if not found_c:
		_fail("'Toggle Camera' must be bound to physical C.", failures)
		return
	for ev in InputMap.action_get_events("Clutch"):
		if ev is InputEventKey and (ev as InputEventKey).physical_keycode == KEY_C:
			_fail("Clutch must no longer use C (moved to X) to avoid double-trigger.", failures)
			return
	print("[OK] toggle action: C -> Toggle Camera, Clutch moved to X.")
