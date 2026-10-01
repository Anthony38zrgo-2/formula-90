extends SceneTree

class TelemetryVehicle extends Node3D:
	var acceleration := Vector3.ZERO
	var steering_amount := 0.0

	func get_true_steering_amount() -> float:
		return steering_amount

	func get_telemetry_snapshot() -> Dictionary:
		return {"lat_g": acceleration.x, "vert_g": acceleration.y, "long_g": acceleration.z}

var failures: Array[String] = []
var vehicle: TelemetryVehicle
var driver_controller: Node
var camera_rig: Node3D
var skeleton: Skeleton3D
var head_modifier: SkeletonModifier3D
var configuration: CockpitCameraConfiguration
var latest_head_pose := Transform3D.IDENTITY
var latest_neck_pose := Transform3D.IDENTITY
var latest_head_local_rotation := Quaternion.IDENTITY
var maximum_bone_length_error := 0.0
var modification_count := 0

func _init() -> void:
	call_deferred("run_validation")

func require_condition(condition: bool, message: String) -> void:
	if not condition:
		failures.append(message)
		printerr(message)

func settle_frames(frame_count: int = 90) -> void:
	for frame_index in range(frame_count):
		await process_frame
	await process_frame
	await process_frame

func run_validation() -> void:
	vehicle = TelemetryVehicle.new()
	vehicle.name = "TelemetryVehicle"
	root.add_child(vehicle)
	var chassis := (load("res://assets/models/vehicles/f1-2030/f1_2030_v10_chassis.glb") as PackedScene).instantiate() as Node3D
	chassis.name = "ChassisVisual"
	vehicle.add_child(chassis)
	var steering_controller := (load("res://scripts/vehicle/steering_wheel_visual_controller.gd") as GDScript).new() as Node
	steering_controller.set("vehicle", vehicle)
	steering_controller.set("chassis_visual", chassis)
	vehicle.add_child(steering_controller)
	driver_controller = (load("res://scripts/vehicle/driver_visual_controller.gd") as GDScript).new() as Node
	driver_controller.name = "DriverVisualController"
	driver_controller.set("driver_model", load("res://assets/models/drivers/driver.glb"))
	driver_controller.set("chassis_visual", chassis)
	driver_controller.set("steering_wheel_controller", steering_controller)
	driver_controller.set("seated_position", Vector3(0.0, -0.011, -0.24))
	driver_controller.set("cockpit_configuration_path", "res://data/cameras/formula_one_2030_cockpit_camera.json")
	vehicle.add_child(driver_controller)
	skeleton = driver_controller.get("driver_skeleton") as Skeleton3D
	head_modifier = driver_controller.get("head_motion_modifier") as SkeletonModifier3D
	if skeleton == null or head_modifier == null:
		printerr("Cockpit requires an imported driver skeleton and a working head modifier.")
		quit(1)
		return
	configuration = head_modifier.get("configuration") as CockpitCameraConfiguration
	head_modifier.modification_processed.connect(record_skeleton_pose)
	camera_rig = (load("res://scenes/runtime/cockpit_camera_rig.tscn") as PackedScene).instantiate() as Node3D
	camera_rig.set("driver_controller_path", NodePath("../TelemetryVehicle/DriverVisualController"))
	camera_rig.set("configuration_path", "res://data/cameras/formula_one_2030_cockpit_camera.json")
	root.add_child(camera_rig)
	await settle_frames()
	var camera := camera_rig.get_node("Camera3D") as Camera3D
	var eye_point := driver_controller.call("get_driver_eye_point") as Node3D
	var driver_instance := driver_controller.get("driver_instance") as Node3D
	var head_geometry := driver_controller.get("driver_head_and_neck") as MeshInstance3D
	var body_geometry := driver_instance.find_child("DriverBody", true, false) as MeshInstance3D
	var glove_geometry := driver_instance.find_child("DriverArticulatedGloves", true, false) as MeshInstance3D
	require_condition(skeleton.get_bone_count() == 55, "Eye point or segmentation changed the driver's bone count.")
	require_condition(head_geometry != null and body_geometry != null and glove_geometry != null, "Cockpit driver geometry is incomplete.")
	require_condition(absf(camera.global_basis.z.y) < 0.001 and absf(camera.global_basis.x.y) < 0.001, "Neutral cockpit does not point at the level horizon.")
	require_condition((-camera.global_basis.z).dot(Vector3.FORWARD) > 0.999, "Cockpit points away from the vehicle's forward direction.")
	var expected_camera_position := eye_point.global_position + eye_point.global_basis.y.normalized() * configuration.viewpoint_elevation_meters
	require_condition(camera.global_position.distance_to(expected_camera_position) < 0.0001, "Cockpit does not apply its configured elevation to the eye point.")
	require_condition(absf(camera.fov - configuration.field_of_view_degrees) < 0.001 and absf(camera.near - configuration.near_clip_distance_meters) < 0.0001, "Cockpit lens does not use its configuration.")
	var neutral_camera_position := camera.global_position
	var camera_configuration := camera_rig.get("configuration") as CockpitCameraConfiguration
	camera_configuration.viewpoint_elevation_meters = 0.0
	camera_rig.call("synchronize_camera")
	require_condition(camera.global_position.distance_to(eye_point.global_position) < 0.0001, "Zero artificial elevation does not restore the authored eye point.")
	camera_configuration.viewpoint_elevation_meters = configuration.viewpoint_elevation_meters
	camera_rig.call("synchronize_camera")
	var neutral_neck_rotation := latest_neck_pose.basis.get_rotation_quaternion()
	var neutral_head_local_rotation := latest_head_local_rotation
	camera_rig.call("set_view_active", true)
	require_condition(not head_geometry.visible and body_geometry.visible and glove_geometry.visible, "Cockpit visibility must hide only the head and neck.")
	camera_rig.call("set_view_active", false)
	require_condition(head_geometry.visible, "External views do not restore the driver's head and neck.")
	vehicle.acceleration = Vector3(2.0, 0.0, 0.0)
	await settle_frames()
	var full_response: Vector2 = head_modifier.get("angular_displacement")
	require_condition(camera.global_position.x < neutral_camera_position.x - 0.005, "Rightward acceleration does not move the driver's eyes toward the left.")
	require_condition(latest_neck_pose.basis.get_rotation_quaternion().angle_to(neutral_neck_rotation) > deg_to_rad(1.0), "Lateral force does not animate the neck.")
	require_condition(latest_head_local_rotation.angle_to(neutral_head_local_rotation) > deg_to_rad(1.0), "Lateral force does not animate the head independently of the neck.")
	expected_camera_position = eye_point.global_position + eye_point.global_basis.y.normalized() * configuration.viewpoint_elevation_meters
	require_condition(camera.global_position.distance_to(expected_camera_position) < 0.0001, "Elevated camera lags behind the modified head.")
	driver_controller.call("set_cockpit_force_response_strength", 0.5)
	await settle_frames()
	var half_response: Vector2 = head_modifier.get("angular_displacement")
	require_condition(absf(half_response.y - full_response.y * 0.5) < 0.0005, "Half force sensitivity does not produce half the head response.")
	driver_controller.call("set_cockpit_force_response_strength", 0.0)
	await settle_frames(5)
	require_condition(camera.global_position.distance_to(neutral_camera_position) < 0.0002, "Zero force sensitivity does not return the eyes to their neutral position.")
	driver_controller.call("set_cockpit_force_response_strength", 1.0)
	vehicle.acceleration = Vector3(0.0, 0.0, -1.5)
	await settle_frames()
	require_condition(camera.global_position.z < neutral_camera_position.z - 0.003, "Braking does not move the driver's head forward.")
	require_condition((-camera.global_basis.z).y < -0.02, "Braking does not nod the driver's gaze downward.")
	vehicle.acceleration = Vector3(0.0, 0.0, 1.5)
	await settle_frames()
	require_condition(camera.global_position.z > neutral_camera_position.z + 0.003, "Acceleration does not move the driver's head backward.")
	require_condition((-camera.global_basis.z).y > 0.02, "Acceleration does not raise the driver's gaze.")
	vehicle.acceleration = Vector3(0.0, 2.0, 0.0)
	await settle_frames()
	require_condition(absf(float(head_modifier.get("angular_displacement").x)) > deg_to_rad(0.5), "Vertical force does not affect head motion.")
	vehicle.acceleration = Vector3(100.0, 100.0, 100.0)
	await settle_frames()
	var limited_displacement: Vector2 = head_modifier.get("angular_displacement")
	require_condition(limited_displacement.is_finite() and absf(limited_displacement.x) <= deg_to_rad(configuration.maximum_pitch_degrees) + 0.0001 and absf(limited_displacement.y) <= deg_to_rad(configuration.maximum_roll_degrees) + 0.0001, "Extreme acceleration exceeds anatomical motion limits.")
	vehicle.acceleration = Vector3.ZERO
	vehicle.position += Vector3(20.0, 10.0, -30.0)
	await settle_frames(4)
	require_condition((head_modifier.get("angular_displacement") as Vector2).length() < 0.0001, "Teleport does not clear residual head motion.")
	vehicle.rotation = Vector3(deg_to_rad(12.0), deg_to_rad(80.0), deg_to_rad(-9.0))
	await settle_frames()
	require_condition(absf(camera.global_basis.z.y) < 0.001 and absf(camera.global_basis.x.y) < 0.001, "Cockpit horizon stabilization fails under chassis pitch and roll.")
	var vehicle_forward := -vehicle.global_basis.z
	vehicle_forward.y = 0.0
	require_condition((-camera.global_basis.z).dot(vehicle_forward.normalized()) > 0.999, "Horizon stabilization loses the vehicle heading.")
	require_condition(maximum_bone_length_error < 0.00001, "Head motion stretches the neck or head bones.")
	require_condition(modification_count > 100, "Head modifier did not process the acceleration sequence.")
	validate_frame_rate_independence()
	var camera_binding_found := false
	for input_event in InputMap.action_get_events("Toggle Camera"):
		if input_event is InputEventKey and input_event.physical_keycode == KEY_C:
			camera_binding_found = true
	require_condition(camera_binding_found, "Physical C is not assigned to the camera cycle.")
	print("COCKPIT_CAMERA_EYE_POSITION=" + str(neutral_camera_position))
	print("COCKPIT_MAXIMUM_BONE_LENGTH_ERROR=" + str(maximum_bone_length_error))
	print("COCKPIT_MODIFICATION_COUNT=" + str(modification_count))
	print("COCKPIT_VALIDATION_FAILURES=" + str(failures.size()))
	quit(0 if failures.is_empty() else 1)

func record_skeleton_pose() -> void:
	modification_count += 1
	var head_index := int(head_modifier.get("head_bone_index"))
	var neck_index := int(head_modifier.get("neck_bone_index"))
	latest_head_pose = skeleton.global_transform * skeleton.get_bone_global_pose(head_index)
	latest_neck_pose = skeleton.global_transform * skeleton.get_bone_global_pose(neck_index)
	latest_head_local_rotation = skeleton.get_bone_pose_rotation(head_index)
	var actual_length := skeleton.get_bone_global_pose(head_index).origin.distance_to(skeleton.get_bone_global_pose(neck_index).origin)
	maximum_bone_length_error = maxf(maximum_bone_length_error, absf(actual_length - skeleton.get_bone_rest(head_index).origin.length()))

func validate_frame_rate_independence() -> void:
	var settled_displacements: Array[Vector2] = []
	for frames_per_second in [30, 60, 120]:
		var modifier := (load("res://scripts/vehicle/driver_head_motion_modifier.gd") as GDScript).new() as SkeletonModifier3D
		modifier.set("configuration", configuration)
		for frame_index in range(frames_per_second):
			modifier.call("update_inertial_motion", Vector3(1.5, 0.5, -1.0), 1.0 / frames_per_second)
		settled_displacements.append(modifier.get("angular_displacement"))
		modifier.free()
	for displacement in settled_displacements:
		require_condition(displacement.distance_to(settled_displacements[0]) < 0.0002, "Head response changes materially with the frame rate.")
