extends SceneTree

const DEFAULT_SCENE_PATH := "res://scenes/runtime/vehicle_test_session_2026.tscn"
const DEFAULT_VEHICLE_ID := "f1_2026_2008"
const EXPECTED_SPAWN := Vector3(-297.652, 0.024, -244.251)
const REQUIRED_GROUPS := [&"Road", &"Curb", &"Grass", &"Gravel", &"Sand", &"Wall", &"Metal"]

func _init() -> void:
	call_deferred("_run")

func _fail(message: String, failures: Array[String]) -> void:
	printerr("[FAIL] " + message)
	failures.append(message)

func _argument(prefix: String, fallback: String) -> String:
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with(prefix):
			return argument.trim_prefix(prefix)
	return fallback

func _run() -> void:
	var failures: Array[String] = []
	var scene_path := _argument("--scene=", DEFAULT_SCENE_PATH)
	var expected_vehicle_id := _argument("--vehicle-id=", DEFAULT_VEHICLE_ID)
	var packed := load(scene_path) as PackedScene
	if packed == null:
		_fail("Fuji runtime scene could not load.", failures)
		quit(1)
		return

	var compositor := packed.instantiate()
	root.add_child(compositor)
	for _frame in 8:
		await process_frame

	var track := compositor.find_child("ActiveTrack", true, false) as Node3D
	var vehicle_root := compositor.find_child("ActiveVehicle", true, false) as Node3D
	var vehicle := compositor.find_child("VehicleRigidBody", true, false) as RigidBody3D
	var camera_rig := compositor.find_child("CameraRig", true, false) as Node3D
	var camera := compositor.find_child("Camera3D", true, false) as Camera3D
	var minimap := compositor.get_node_or_null("DisplayAspect/DisplayStage/HudLayer/DebugHud/Minimap")
	if track == null or vehicle == null:
		_fail("Fuji track or vehicle was not composed.", failures)
	else:
		var marker := track.find_child("VehicleSpawn", true, false) as Marker3D
		if marker == null or marker.global_position.distance_to(EXPECTED_SPAWN) > 0.05:
			_fail("VehicleSpawn does not match Fuji package metadata.", failures)
	if vehicle_root == null or vehicle_root.scene_file_path.find(expected_vehicle_id) < 0:
		_fail("Expected vehicle '%s' is not the active model." % expected_vehicle_id, failures)
	if camera_rig == null or camera == null or not camera.current:
		_fail("Canonical chase camera is missing or inactive.", failures)

	for group in REQUIRED_GROUPS:
		var found := false
		for node in get_nodes_in_group(group):
			if track != null and track.is_ancestor_of(node):
				found = true
				break
		if not found:
			_fail("Missing Fuji collision group: %s" % group, failures)

	if minimap == null or minimap.get("map_data") == null or not minimap.get("map_data").is_valid_map():
		_fail("Fuji minimap data is missing or invalid.", failures)

	for _frame in 180:
		await physics_frame
	if vehicle != null:
		var contacts := 0
		for ray_name in [
			"RayCast_FL_In", "RayCast_FL_Mid", "RayCast_FL_Out",
			"RayCast_FR_In", "RayCast_FR_Mid", "RayCast_FR_Out",
			"RayCast_RL_In", "RayCast_RL_Mid", "RayCast_RL_Out",
			"RayCast_RR_In", "RayCast_RR_Mid", "RayCast_RR_Out",
		]:
			var ray := vehicle.find_child(ray_name, true, false) as RayCast3D
			if ray != null and ray.is_colliding():
				contacts += 1
		if contacts < 4:
			_fail("Only %d/12 wheel rays contact Fuji after settling." % contacts, failures)
		if vehicle.global_position.y < -5.0:
			_fail("Vehicle fell through Fuji collision.", failures)
		if camera_rig != null:
			var vertical_offset := camera_rig.global_position.y - vehicle.global_position.y
			if vertical_offset < 2.0 or vertical_offset > 7.0:
				_fail("Chase camera vertical tracking is outside its valid range: %.2f m." % vertical_offset, failures)
			var vehicle_forward := -vehicle.global_transform.basis.z.normalized()
			var camera_delta := camera_rig.global_position - vehicle.global_position
			if camera_delta.dot(vehicle_forward) >= -2.0:
				_fail("Chase camera is not positioned behind the active vehicle.", failures)
			var lifted_position := vehicle.global_position + Vector3(0.0, 10.0, 0.0)
			vehicle.reset_vehicle(lifted_position, vehicle.global_rotation.y)
			for _camera_frame in 3:
				await process_frame
			vertical_offset = camera_rig.global_position.y - vehicle.global_position.y
			if vertical_offset < 2.0 or vertical_offset > 7.0:
				_fail("Chase camera did not follow a 10 m elevation change: %.2f m." % vertical_offset, failures)

	compositor.queue_free()
	if failures.is_empty():
		print("[PASS] %s loads on Fuji 76-77 with spawn, surfaces, camera, HUD and wheel contact." % expected_vehicle_id)
	quit(failures.size())
