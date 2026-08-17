extends SceneTree

const SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"
const SETTLE_FRAMES := 180

func _init() -> void:
	call_deferred("_run")

func _run() -> void:
	var packed := load(SCENE_PATH) as PackedScene
	if packed == null:
		printerr("[FAIL] scene missing")
		quit(1)
		return
	var root_node = packed.instantiate()
	root.add_child(root_node)
	for _i in range(4):
		await process_frame
	var vehicle = root_node.find_child("VehicleRigidBody", true, false)
	if vehicle == null:
		printerr("[FAIL] VehicleRigidBody missing")
		quit(1)
		return
	vehicle.enable_player_input = false
	vehicle.throttle_amount = 0.0
	var spawn_marker = root_node.find_child("VehicleSpawn", true, false)
	print("[INFO] runtime spawn_marker=%s default_spawn_height=%.6f vehicle_y=%.6f" % [
		str(spawn_marker.global_position if spawn_marker != null else Vector3.ZERO),
		float(vehicle.get_default_spawn_height()), vehicle.global_position.y])
	var file := FileAccess.open("user://f1_94_spawn_settle_forensics.csv", FileAccess.WRITE)
	file.store_line("frame,time_ms,body_x,body_y,body_z,total_speed_kmh,forward_kmh,lateral_kmh,vertical_kmh,yaw_rate_rad_s,lat_g,long_g,vert_g,fl_comp,fr_comp,rl_comp,rr_comp")
	var initial_y: float = vehicle.global_position.y
	var settled_y: float = initial_y
	var max_abs_lateral: float = 0.0
	var max_abs_yaw_rate: float = 0.0
	for frame in range(SETTLE_FRAMES + 1):
		await physics_frame
		var basis: Basis = vehicle.global_transform.basis
		var linear_velocity: Vector3 = vehicle.linear_velocity
		var forward_velocity: float = -basis.z.dot(linear_velocity)
		var lateral_velocity: float = basis.x.dot(linear_velocity)
		var vertical_velocity: float = linear_velocity.y
		var comp = vehicle.get_wheel_compressions()
		var yaw_rate: float = vehicle.angular_velocity.y
		max_abs_lateral = maxf(max_abs_lateral, absf(lateral_velocity))
		max_abs_yaw_rate = maxf(max_abs_yaw_rate, absf(yaw_rate))
		if frame == SETTLE_FRAMES:
			settled_y = vehicle.global_position.y
		if frame % 5 == 0 or frame == SETTLE_FRAMES:
			file.store_line("%d,%d,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.3f,%.3f,%.3f,%.3f" % [
				frame, int(frame * 1000.0 / Engine.physics_ticks_per_second),
				vehicle.global_position.x, vehicle.global_position.y, vehicle.global_position.z,
				linear_velocity.length() * 3.6, forward_velocity * 3.6, lateral_velocity * 3.6, vertical_velocity * 3.6,
				yaw_rate, vehicle.lat_g, vehicle.long_g, vehicle.vert_g,
				comp[0], comp[1], comp[2], comp[3]])
	file.close()
	print("[OK] spawn settle: initial_y=%.6f settled_y=%.6f delta=%.6f max_lateral=%.6f m/s max_yaw_rate=%.6f rad/s" % [
		initial_y, settled_y, settled_y - initial_y, max_abs_lateral, max_abs_yaw_rate])
	quit(0)
