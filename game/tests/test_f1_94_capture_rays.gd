extends SceneTree

const SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"

func _init() -> void:
	call_deferred("_run")

func _run() -> void:
	var packed := load(SCENE_PATH) as PackedScene
	if packed == null:
		printerr("[FAIL] scene missing")
		quit(1)
		return
	var car = packed.instantiate()
	root.add_child(car)
	for _i in range(8):
		await physics_frame
	var vehicle = car.find_child("VehicleRigidBody", true, false)
	if vehicle == null:
		printerr("[FAIL] VehicleRigidBody missing")
		quit(1)
		return
	var file := FileAccess.open("user://f1_94_ray_capture.csv", FileAccess.WRITE)
	file.store_line("frame,wheel,ray,origin_y,collision_y,distance,colliding,anchor_x,anchor_y,anchor_z,ray_length")
	var wheels := ["FL", "FR", "RL", "RR"]
	var rays := ["In", "Mid", "Out"]
	for capture_frame in [8, 128]:
		if capture_frame == 128:
			for _i in range(120):
				await physics_frame
		for wheel_index in range(4):
			var anchor: Vector3 = vehicle.get_wheel_anchor_local(wheel_index)
			var ray_length: float = vehicle.get_ray_length(wheel_index)
			for ray_name in rays:
				var ray = vehicle.find_child("RayCast_%s_%s" % [wheels[wheel_index], ray_name], true, false)
				if ray == null:
					continue
				var hit_y: float = ray.get_collision_point().y if ray.is_colliding() else 0.0
				var distance: float = ray.global_position.distance_to(ray.get_collision_point()) if ray.is_colliding() else -1.0
				file.store_line("%d,%s,%s,%.6f,%.6f,%.6f,%s,%.6f,%.6f,%.6f,%.6f" % [
					capture_frame, wheels[wheel_index], ray_name, ray.global_position.y, hit_y, distance,
					str(ray.is_colliding()), anchor.x, anchor.y, anchor.z, ray_length])
	file.close()
	print("[OK] ray capture written to user://f1_94_ray_capture.csv")
	quit(0)
