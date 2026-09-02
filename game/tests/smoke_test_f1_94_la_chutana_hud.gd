extends SceneTree

const SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"

const EXPECTED_WHEEL_NODES := {
	"FrontLeftWheel": "res://assets/models/vehicles/f1_94/decoupled/geometry/F1_94_wheel_front_geometry.glb",
	"FrontRightWheel": "res://assets/models/vehicles/f1_94/decoupled/geometry/F1_94_wheel_front_geometry.glb",
	"RearLeftWheel": "res://assets/models/vehicles/f1_94/decoupled/geometry/F1_94_wheel_rear_geometry.glb",
	"RearRightWheel": "res://assets/models/vehicles/f1_94/decoupled/geometry/F1_94_wheel_rear_geometry.glb",
}

const EXPECTED_RAYS := [
	"RayCast_FL_In", "RayCast_FL_Mid", "RayCast_FL_Out",
	"RayCast_FR_In", "RayCast_FR_Mid", "RayCast_FR_Out",
	"RayCast_RL_In", "RayCast_RL_Mid", "RayCast_RL_Out",
	"RayCast_RR_In", "RayCast_RR_Mid", "RayCast_RR_Out"
]

func _init() -> void:
	call_deferred("_run")

func _fail(message: String, failures: Array[String]) -> void:
	printerr("[FAIL] " + message)
	failures.append(message)

func _run() -> void:
	var failures: Array[String] = []
	var packed := load(SCENE_PATH) as PackedScene
	if packed == null:
		_fail("F1-94 La Chutana scene could not load.", failures)
		quit(1)
		return

	var compositor := packed.instantiate()
	root.add_child(compositor)
	for _frame in 4:
		await process_frame

	var world := compositor.get_node_or_null("WorldViewport/RaceSession")
	var vehicle := compositor.get_node_or_null("WorldViewport/RaceSession/VehicleContainer/ActiveVehicle/VehicleRigidBody") as RigidBody3D
	var core := compositor.get_node_or_null("F90Core")
	var hud := compositor.get_node_or_null("DisplayAspect/DisplayStage/HudLayer/DebugHud")
	var minimap := compositor.get_node_or_null("DisplayAspect/DisplayStage/HudLayer/DebugHud/Minimap")
	var speed_gauge := compositor.get_node_or_null("DisplayAspect/DisplayStage/HudLayer/DebugHud/SpeedGauge")
	var retro_hud := compositor.get_node_or_null("DisplayAspect/DisplayStage/HudLayer/DebugHud/RetroHud")
	var aids := compositor.get_node_or_null("WorldViewport/RaceSession/DrivingAids")

	if world == null or vehicle == null or compositor.get_node_or_null("WorldViewport").get_camera_3d() == null:
		_fail("3D world, F194 vehicle, or active camera is missing from the compositor.", failures)
	if core == null or not core.has_method("is_engine_loaded") or not core.is_engine_loaded():
		_fail("F90Core did not initialize. Check the GDExtension/Rust core ABI contract.", failures)
	if vehicle != null and vehicle.has_method("get_brake_state_snapshot"):
		var brake_snapshot: Dictionary = vehicle.get_brake_state_snapshot()
		for wheel_name in ["FL", "FR", "RL", "RR"]:
			var wheel: Dictionary = brake_snapshot.get(wheel_name, {})
			if not wheel.has("disc_c") or not wheel.has("rim_c"):
				_fail("Two-node brake telemetry missing for %s." % wheel_name, failures)
			if not wheel.has("efficiency"):
				_fail("Brake efficiency telemetry missing for %s." % wheel_name, failures)
	if hud == null or minimap == null or (speed_gauge == null and retro_hud == null):
		_fail("HUD, minimap, or speed gauge/retro HUD was not extracted into HudLayer.", failures)
	if hud != null and hud.get("_vehicle") != vehicle:
		_fail("HUD is not directly bound to F194/VehicleRigidBody.", failures)
	if hud != null and hud.get("_aids") != aids:
		_fail("HUD is not directly bound to DrivingAids.", failures)
	if minimap != null:
		if minimap.get("map_data") == null or not minimap.get("map_data").is_valid_map():
			_fail("La Chutana minimap data is missing or invalid.", failures)
		if minimap.get("_target") != vehicle:
			_fail("Minimap is not directly bound to F194/VehicleRigidBody.", failures)
	if world != null and world.get_node_or_null("DebugHud") != null:
		_fail("DebugHud remained inside the filtered 3D world.", failures)

	if vehicle != null:
		for wheel_name in EXPECTED_WHEEL_NODES:
			var w_node := vehicle.get_node_or_null(wheel_name) as Node3D
			if w_node == null:
				_fail("Missing wheel node: " + wheel_name, failures)
				continue
			var visual := w_node.find_child("Visual", true, false)
			if visual == null:
				_fail("Missing wheel visual inside " + wheel_name, failures)
				continue
			if visual.scene_file_path != EXPECTED_WHEEL_NODES[wheel_name]:
				_fail("Independent wheel asset mismatch: " + wheel_name, failures)

		for ray_name in EXPECTED_RAYS:
			var ray := vehicle.find_child(ray_name, true, false) as RayCast3D
			if ray == null:
				_fail("Missing 12-raycast sensor: " + ray_name, failures)

	for _frame in 180:
		await physics_frame

	if vehicle != null and (not vehicle.has_method("get_speed_kmh") or not vehicle.has_method("get_motor_rpm")):
		_fail("VehicleRigidBody did not expose the F194 Rust physics API; its DLL did not initialize.", failures)

	if vehicle != null:
		var contacts := 0
		for ray_name in EXPECTED_RAYS:
			var ray := vehicle.find_child(ray_name, true, false) as RayCast3D
			if ray != null and ray.is_colliding():
				contacts += 1
		if contacts < 4:
			_fail("Only %d/12 RayCasts contact La Chutana after settling." % contacts, failures)

	if minimap != null and vehicle != null:
		var map_before: Vector2 = minimap.call("get_player_map_position")
		vehicle.freeze = true
		vehicle.global_position.x += 2.0
		await process_frame
		var map_after: Vector2 = minimap.call("get_player_map_position")
		if map_before.distance_to(map_after) < 0.01:
			_fail("Minimap player marker does not react to vehicle movement.", failures)

	compositor.queue_free()
	if failures.is_empty():
		print("[PASS] F1-94 loads on La Chutana with live sensors, HUD, and minimap.")
	quit(failures.size())
