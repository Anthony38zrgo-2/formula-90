extends SceneTree

class TelemetryVehicle extends Node3D:
	var acceleration := Vector3.ZERO
	var steering_amount := 0.0
	var wheel_compressions := PackedFloat64Array([50.0, 50.0, 50.0, 50.0])
	var raycasts: Array = []

	func get_true_steering_amount() -> float:
		return steering_amount

	func get_telemetry_snapshot() -> Dictionary:
		return {"lat_g": acceleration.x, "vert_g": acceleration.y, "long_g": acceleration.z, "wheel_compressions": wheel_compressions}

	func get_raycast_list() -> Array:
		return raycasts

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
	camera_rig.set("load_saved_preferences", false)
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
	var expected_camera_position := (camera_rig.call("get_rendered_eye_transform") as Transform3D).origin + (camera_rig.call("get_viewpoint_elevation_direction") as Vector3) * configuration.viewpoint_elevation_meters
	require_condition(camera.global_position.distance_to(expected_camera_position) < 0.0001, "Cockpit does not apply its configured elevation to the eye point.")
	require_condition(absf(camera.fov - configuration.field_of_view_degrees) < 0.001 and absf(camera.near - configuration.near_clip_distance_meters) < 0.0001, "Cockpit lens does not use its configuration.")
	var neutral_camera_position := camera.global_position
	var camera_configuration := camera_rig.get("configuration") as CockpitCameraConfiguration
	var original_viewpoint_elevation := configuration.viewpoint_elevation_meters
	camera_configuration.viewpoint_elevation_meters = 0.0
	camera_rig.call("synchronize_camera")
	require_condition(camera.global_position.distance_to(eye_point.global_position) < 0.0001, "Zero artificial elevation does not restore the authored eye point.")
	camera_configuration.viewpoint_elevation_meters = original_viewpoint_elevation
	camera_rig.call("synchronize_camera")
	var neutral_neck_rotation := latest_neck_pose.basis.get_rotation_quaternion()
	var neutral_head_local_rotation := latest_head_local_rotation
	camera_rig.call("set_view_active", true)
	require_condition(not head_geometry.visible and body_geometry.visible and glove_geometry.visible, "Cockpit visibility must hide only the head and neck.")
	camera_rig.call("set_view_active", false)
	require_condition(head_geometry.visible, "External views do not restore the driver's head and neck.")
	vehicle.acceleration = Vector3(2.0, 0.0, 0.0)
	await settle_frames()
	require_condition((head_modifier.get("angular_displacement") as Vector2).length() < 0.00001 and camera.global_position.distance_to(neutral_camera_position) < 0.0001, "Cornering gravity independently moves the cockpit head or eye point.")
	vehicle.wheel_compressions = PackedFloat64Array([70.0, 50.0, 70.0, 50.0])
	await settle_frames(5)
	var full_response: Vector2 = head_modifier.get("angular_displacement")
	require_condition(absf(full_response.y) > deg_to_rad(0.05), "A one-sided suspension bump does not animate lateral head motion.")
	require_condition(latest_neck_pose.basis.get_rotation_quaternion().angle_to(neutral_neck_rotation) > absf(full_response.y) * configuration.neck_rotation_fraction * 0.9, "A suspension bump does not animate the neck with its configured share of motion.")
	require_condition(latest_head_local_rotation.angle_to(neutral_head_local_rotation) > absf(full_response.y) * (1.0 - configuration.neck_rotation_fraction) * 0.9, "A suspension bump does not animate the head independently with its configured share of motion.")
	expected_camera_position = (camera_rig.call("get_rendered_eye_transform") as Transform3D).origin + (camera_rig.call("get_viewpoint_elevation_direction") as Vector3) * configuration.viewpoint_elevation_meters + (camera_rig.get("positional_correction_world") as Vector3) * camera_configuration.positional_stabilization_strength
	require_condition(camera.global_position.distance_to(expected_camera_position) < 0.0001, "Elevated camera loses the animated eye and bounded bump correction.")
	vehicle.wheel_compressions = PackedFloat64Array([50.0, 50.0, 50.0, 50.0])
	await settle_frames()
	head_modifier.call("reset_motion")
	await settle_frames(2)
	driver_controller.call("set_cockpit_force_response_strength", 0.5)
	vehicle.wheel_compressions = PackedFloat64Array([70.0, 50.0, 70.0, 50.0])
	await settle_frames(5)
	var half_response: Vector2 = head_modifier.get("angular_displacement")
	require_condition(absf(half_response.y - full_response.y * 0.5) < 0.0005, "Half force sensitivity does not produce half the head response.")
	driver_controller.call("set_cockpit_force_response_strength", 0.0)
	await settle_frames(25)
	require_condition(camera.global_position.distance_to(neutral_camera_position) < 0.0002, "Zero force sensitivity does not return the eyes to their neutral position.")
	driver_controller.call("set_cockpit_force_response_strength", 1.0)
	vehicle.acceleration = Vector3.ZERO
	vehicle.wheel_compressions = PackedFloat64Array([50.0, 50.0, 50.0, 50.0])
	await settle_frames()
	vehicle.acceleration = Vector3(0.0, 0.0, -3.0)
	await settle_frames(10)
	require_condition(camera.global_position.z < neutral_camera_position.z - 0.001, "Initial braking does not move the driver's head forward.")
	require_condition((-camera.global_basis.z).y < -0.015, "Initial braking does not nod the driver's gaze downward.")
	await settle_frames()
	require_condition(absf((-camera.global_basis.z).y) < 0.004, "Sustained braking does not recover the horizon.")
	vehicle.acceleration = Vector3(0.0, 0.0, 3.0)
	await settle_frames(10)
	require_condition(camera.global_position.z > neutral_camera_position.z + 0.001, "Initial acceleration does not move the driver's head backward.")
	require_condition((-camera.global_basis.z).y > 0.015, "Initial acceleration does not raise the driver's gaze.")
	await settle_frames()
	require_condition(absf((-camera.global_basis.z).y) < 0.004, "Sustained acceleration does not recover the horizon.")
	vehicle.acceleration = Vector3(0.0, 2.0, 0.0)
	await settle_frames()
	require_condition((head_modifier.get("angular_displacement") as Vector2).length() < 0.00001, "Vertical gravity from a change of grade moves the head.")
	vehicle.wheel_compressions = PackedFloat64Array([75.0, 75.0, 75.0, 75.0])
	await settle_frames(5)
	var shared_motion := head_modifier.get("motion_state") as RefCounted
	require_condition(absf(float(shared_motion.get("vertical_displacement_meters"))) > 0.00001, "A symmetric suspension bump does not retain vertical feedback.")
	require_condition(absf(float(head_modifier.get("angular_displacement").x)) < deg_to_rad(0.01), "A symmetric suspension bump incorrectly nods the driver's gaze.")
	vehicle.acceleration = Vector3(100.0, 100.0, 100.0)
	await settle_frames()
	var limited_displacement: Vector2 = head_modifier.get("angular_displacement")
	require_condition(limited_displacement.is_finite() and absf(limited_displacement.x) <= deg_to_rad(configuration.maximum_pitch_degrees) + 0.0001 and absf(limited_displacement.y) <= deg_to_rad(configuration.maximum_roll_degrees) + 0.0001, "Extreme acceleration exceeds anatomical motion limits.")
	vehicle.acceleration = Vector3.ZERO
	vehicle.wheel_compressions = PackedFloat64Array([50.0, 50.0, 50.0, 50.0])
	vehicle.position += Vector3(20.0, 10.0, -30.0)
	await settle_frames(4)
	require_condition((head_modifier.get("angular_displacement") as Vector2).length() < 0.0001, "Teleport does not clear residual head motion.")
	vehicle.rotation = Vector3(deg_to_rad(12.0), deg_to_rad(80.0), deg_to_rad(-9.0))
	await settle_frames()
	require_condition(camera.global_basis.y.dot(vehicle.global_basis.y) > 0.99, "Cockpit reference does not follow a sustained grade and banking.")
	var vehicle_forward := -vehicle.global_basis.z
	vehicle_forward.y = 0.0
	require_condition((-camera.global_basis.z).dot((-vehicle.global_basis.z).normalized()) > 0.99, "Road following loses the vehicle heading.")
	vehicle.acceleration = Vector3(3.0, -3.0, 0.0)
	for grade_frame in range(120):
		vehicle.position += Vector3(0.005, 0.01, -0.3)
		vehicle.rotation.x = deg_to_rad(12.0) * sin(float(grade_frame) / 120.0 * TAU)
		await process_frame
	await settle_frames(2)
	require_condition((head_modifier.get("angular_displacement") as Vector2).length() < 0.00001, "A crest or corner introduces additional gravity-driven head motion.")
	require_condition((camera_rig.get("positional_correction_world") as Vector3).length() < 0.00001, "Grade or corner travel sinks or raises the cockpit with a positional correction.")
	require_condition(maximum_bone_length_error < 0.00001, "Head motion stretches the neck or head bones.")
	require_condition(modification_count > 100, "Head modifier did not process the acceleration sequence.")
	validate_frame_rate_independence()
	validate_longitudinal_compensation()
	validate_gear_shift_motion_suppression()
	validate_bump_filtering()
	validate_suspension_bump_separation()
	validate_vertical_bump_smoothness()
	validate_positional_stabilization()
	validate_shared_motion_sampling()
	await validate_camera_chassis_synchronization()
	validate_road_reference_and_velocity_alignment()
	await validate_road_bump_confirmation()
	await validate_motion_preferences()
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
		var modifier := (load("res://scripts/camera/cockpit_driver_motion_state.gd") as GDScript).new() as RefCounted
		modifier.set("configuration", configuration)
		for frame_index in range(frames_per_second):
			modifier.call("update_inertial_motion", Vector3(1.5, 0.5, -1.0), 1.0 / frames_per_second)
		settled_displacements.append(modifier.get("angular_displacement"))
	for displacement in settled_displacements:
		require_condition(displacement.distance_to(settled_displacements[0]) < 0.0002, "Head response changes materially with the frame rate.")

func make_motion_modifier(previous_response: bool = false) -> RefCounted:
	var modifier := (load("res://scripts/camera/cockpit_driver_motion_state.gd") as GDScript).new() as RefCounted
	var motion_configuration := CockpitCameraConfiguration.load_from_path("res://data/cameras/formula_one_2030_cockpit_camera.json")
	if previous_response:
		motion_configuration.longitudinal_compensation_strength = 0.0
		motion_configuration.lateral_acceleration_filter_time_seconds = 0.0
		motion_configuration.vertical_acceleration_filter_time_seconds = 0.0
		motion_configuration.lateral_response_frequency_hertz = motion_configuration.response_frequency_hertz
		motion_configuration.vertical_response_frequency_hertz = motion_configuration.response_frequency_hertz
		motion_configuration.damping_ratio = 0.85
		motion_configuration.lateral_damping_ratio = 0.85
		motion_configuration.vertical_damping_ratio = 0.85
	modifier.set("configuration", motion_configuration)
	return modifier

func validate_longitudinal_compensation() -> void:
	for acceleration_direction in [-1.0, 1.0]:
		var modifier := make_motion_modifier()
		var peak_pitch := 0.0
		var peak_frame := 0
		for frame_index in range(90):
			modifier.call("update_inertial_motion", Vector3(0.0, 0.0, acceleration_direction * 3.0), 1.0 / 60.0)
			var pitch := absf(float(modifier.get("angular_displacement").x))
			if pitch > peak_pitch:
				peak_pitch = pitch
				peak_frame = frame_index + 1
		var recovered_pitch := absf(float(modifier.get("angular_displacement").x))
		require_condition(peak_pitch > deg_to_rad(0.8) and peak_pitch < deg_to_rad(1.4) and peak_frame >= 5 and peak_frame <= 18, "Longitudinal transfer does not preserve a bounded initial impulse.")
		require_condition(recovered_pitch < deg_to_rad(0.2), "Longitudinal transfer does not recover the horizon after 1.5 seconds.")
		for release_frame in range(30):
			modifier.call("update_inertial_motion", Vector3.ZERO, 1.0 / 60.0)
			require_condition(float(modifier.get("angular_displacement").x) * acceleration_direction < deg_to_rad(0.05), "Releasing longitudinal force produces an opposite head kick.")
		modifier.call("reset_motion")
		require_condition(float(modifier.get("longitudinal_compensation_gravity")) == 0.0 and float(modifier.get("filtered_longitudinal_acceleration")) == 0.0 and float(modifier.get("gear_shift_motion_envelope")) == 0.0 and (modifier.get("filtered_bump_acceleration") as Vector2) == Vector2.ZERO, "Motion reset retains compensation or filtered acceleration.")
		print("COCKPIT_LONGITUDINAL_RESPONSE=" + str({"direction": acceleration_direction, "peak_degrees": rad_to_deg(peak_pitch), "peak_seconds": float(peak_frame) / 60.0, "recovered_degrees": rad_to_deg(recovered_pitch)}))
	var reversing_modifier := make_motion_modifier()
	for settle_frame in range(90):
		reversing_modifier.call("update_inertial_motion", Vector3(0.0, 0.0, -3.0), 1.0 / 60.0)
	for transition_frame in range(12):
		reversing_modifier.call("update_inertial_motion", Vector3(0.0, 0.0, 3.0), 1.0 / 60.0)
	require_condition(float(reversing_modifier.get("angular_displacement").x) < -deg_to_rad(0.8), "Reversing longitudinal force does not create a new transfer impulse.")

func validate_gear_shift_motion_suppression() -> void:
	for acceleration_direction in [-1.0, 1.0]:
		var unattenuated_motion := make_motion_modifier()
		var attenuated_motion := make_motion_modifier()
		(unattenuated_motion.get("configuration") as CockpitCameraConfiguration).gear_shift_motion_reduction_strength = 0.0
		var motion_states := [unattenuated_motion, attenuated_motion]
		var peak_pitch := [0.0, 0.0]
		var peak_pitch_step := [0.0, 0.0]
		for motion_state in motion_states:
			motion_state.call("update_sample", {"gear": 2, "long_g": 0.0}, Transform3D.IDENTITY, Vector3.UP, 1.0 / 120.0, 0.0)
			require_condition(float(motion_state.get("gear_shift_motion_envelope")) == 0.0, "Reading the initial gear triggers an artificial gear-change impulse.")
		for frame_index in range(90):
			var longitudinal_acceleration: float = acceleration_direction * 3.0 if frame_index < 12 else 0.0
			for motion_index in range(motion_states.size()):
				var motion_state: RefCounted = motion_states[motion_index]
				var previous_pitch := float(motion_state.get("angular_displacement").x)
				motion_state.call("update_sample", {"gear": 3, "long_g": longitudinal_acceleration}, Transform3D.IDENTITY, Vector3.UP, 1.0 / 120.0, 0.0)
				var current_pitch := float(motion_state.get("angular_displacement").x)
				peak_pitch[motion_index] = maxf(peak_pitch[motion_index], absf(current_pitch))
				peak_pitch_step[motion_index] = maxf(peak_pitch_step[motion_index], absf(current_pitch - previous_pitch))
		require_condition(peak_pitch[1] < peak_pitch[0] * 0.65 and peak_pitch_step[1] < peak_pitch_step[0] * 0.7, "A gear change does not reduce the camera impulse and its fastest movement.")
		require_condition(absf(float(attenuated_motion.get("angular_displacement").x)) < deg_to_rad(0.05), "Gear change damping retains a head offset after the transient ends.")
		print("COCKPIT_GEAR_SHIFT_IMPULSE_RATIO=" + str({"direction": acceleration_direction, "pitch": peak_pitch[1] / peak_pitch[0], "fastest_step": peak_pitch_step[1] / peak_pitch_step[0]}))
	var sustained_motion := make_motion_modifier()
	sustained_motion.call("update_sample", {"gear": 2, "long_g": 0.0}, Transform3D.IDENTITY, Vector3.UP, 1.0 / 120.0, 0.0)
	for acceleration_frame in range(72):
		sustained_motion.call("update_sample", {"gear": 2, "long_g": 3.0}, Transform3D.IDENTITY, Vector3.UP, 1.0 / 120.0, 0.0)
	var pitch_before_shift := absf(float(sustained_motion.get("angular_displacement").x))
	var maximum_rebound := 0.0
	var maximum_opposite_pitch := 0.0
	for shift_frame in range(72):
		sustained_motion.call("update_sample", {"gear": 3, "long_g": 3.0}, Transform3D.IDENTITY, Vector3.UP, 1.0 / 120.0, 0.0)
		maximum_rebound = maxf(maximum_rebound, absf(float(sustained_motion.get("angular_displacement").x)))
		maximum_opposite_pitch = maxf(maximum_opposite_pitch, float(sustained_motion.get("angular_displacement").x))
	require_condition(maximum_rebound <= pitch_before_shift + deg_to_rad(0.05) and maximum_opposite_pitch < deg_to_rad(0.05), "A gear change under sustained acceleration generates a second camera kick.")
	print("COCKPIT_SUSTAINED_SHIFT_REBOUND_DEGREES=" + str({"before": rad_to_deg(pitch_before_shift), "after_maximum": rad_to_deg(maximum_rebound), "opposite": rad_to_deg(maximum_opposite_pitch)}))

func validate_bump_filtering() -> void:
	for frames_per_second in [30, 60, 120]:
		var previous_modifier := make_motion_modifier(true)
		var filtered_modifier := make_motion_modifier()
		var previous_energy := Vector2.ZERO
		var filtered_energy := Vector2.ZERO
		for frame_index in range(frames_per_second * 2):
			var elapsed_time := float(frame_index + 1) / float(frames_per_second)
			var bump_acceleration := Vector3(3.0, 4.0, 0.0) * sin(TAU * 8.0 * elapsed_time)
			previous_modifier.call("update_inertial_motion", bump_acceleration, 1.0 / frames_per_second)
			filtered_modifier.call("update_inertial_motion", bump_acceleration, 1.0 / frames_per_second)
			if elapsed_time >= 0.5:
				var previous_displacement := Vector2(float(previous_modifier.get("vertical_displacement_meters")), float(previous_modifier.get("angular_displacement").y))
				var filtered_displacement := Vector2(float(filtered_modifier.get("vertical_displacement_meters")), float(filtered_modifier.get("angular_displacement").y))
				previous_energy += previous_displacement * previous_displacement
				filtered_energy += filtered_displacement * filtered_displacement
		var motion_ratio := Vector2(sqrt(filtered_energy.x / previous_energy.x), sqrt(filtered_energy.y / previous_energy.y))
		require_condition(motion_ratio.x < 0.6 and motion_ratio.y < 0.6, "Bump filtering does not attenuate vertical and lateral angular shake.")
		print("COCKPIT_BUMP_ANGULAR_RESPONSE_RATIO=" + str([frames_per_second, motion_ratio]))

func validate_suspension_bump_separation() -> void:
	for frames_per_second in [30, 60, 120]:
		var modifier := make_motion_modifier()
		var telemetry_vehicle := TelemetryVehicle.new()
		modifier.set("vehicle", telemetry_vehicle)
		telemetry_vehicle.acceleration = Vector3(4.0, -4.0, 0.0)
		var elapsed_seconds := 1.0 / float(frames_per_second)
		for frame_index in range(frames_per_second):
			var acceleration: Vector3 = modifier.call("read_vehicle_acceleration", elapsed_seconds)
			require_condition(acceleration == Vector3.ZERO, "Corner or crest gravity enters the suspension bump channels.")
		for frame_index in range(frames_per_second * 2):
			var elapsed_time := float(frame_index + 1) / float(frames_per_second)
			telemetry_vehicle.wheel_compressions = PackedFloat64Array([50.0 + elapsed_time * 5.0, 50.0 - elapsed_time * 5.0, 50.0 + elapsed_time * 5.0, 50.0 - elapsed_time * 5.0])
			var acceleration: Vector3 = modifier.call("read_vehicle_acceleration", elapsed_seconds)
			require_condition(acceleration == Vector3.ZERO, "Slow suspension travel from a grade or weight transfer is interpreted as a bump.")
		modifier.call("reset_motion")
		telemetry_vehicle.wheel_compressions = PackedFloat64Array([50.0, 50.0, 50.0, 50.0])
		modifier.call("read_vehicle_acceleration", elapsed_seconds)
		var peak_motion := Vector2.ZERO
		for frame_index in range(frames_per_second * 2):
			var elapsed_time := float(frame_index + 1) / float(frames_per_second)
			var bump_displacement := 15.0 * sin(TAU * 8.0 * elapsed_time)
			telemetry_vehicle.wheel_compressions = PackedFloat64Array([50.0 + bump_displacement, 50.0, 50.0 + bump_displacement, 50.0])
			var acceleration: Vector3 = modifier.call("read_vehicle_acceleration", elapsed_seconds)
			modifier.call("update_inertial_motion", acceleration, elapsed_seconds)
			var displacement := Vector2(float(modifier.get("vertical_displacement_meters")), float(modifier.get("angular_displacement").y))
			peak_motion.x = maxf(peak_motion.x, absf(displacement.x))
			peak_motion.y = maxf(peak_motion.y, absf(displacement.y))
		require_condition(peak_motion.x > 0.000001 and peak_motion.y > deg_to_rad(0.005), "Real suspension bumps do not retain vertical and lateral head response.")
		telemetry_vehicle.wheel_compressions = PackedFloat64Array([50.0, 50.0, 50.0, 50.0])
		for settle_frame in range(frames_per_second * 2):
			modifier.call("update_inertial_motion", modifier.call("read_vehicle_acceleration", elapsed_seconds), elapsed_seconds)
		require_condition((modifier.get("angular_displacement") as Vector2).length() < deg_to_rad(0.01), "Suspension bump motion does not settle at the horizon.")
		print("COCKPIT_SUSPENSION_BUMP_PEAK=" + str({"frames_per_second": frames_per_second, "vertical_meters": peak_motion.x, "roll_degrees": rad_to_deg(peak_motion.y)}))
		telemetry_vehicle.wheel_compressions = PackedFloat64Array([NAN, 50.0, 50.0, 50.0])
		require_condition((modifier.call("read_vehicle_acceleration", elapsed_seconds) as Vector3) == Vector3.ZERO, "Invalid suspension telemetry introduces camera movement.")
		telemetry_vehicle.wheel_compressions = PackedFloat64Array()
		require_condition((modifier.call("read_vehicle_acceleration", elapsed_seconds) as Vector3) == Vector3.ZERO, "Absent suspension telemetry falls back to lateral or vertical gravity.")
		telemetry_vehicle.free()

func validate_vertical_bump_smoothness() -> void:
	for frames_per_second in [30, 60, 120]:
		var previous_modifier := make_motion_modifier()
		var smoother_modifier := make_motion_modifier()
		var previous_configuration := previous_modifier.get("configuration") as CockpitCameraConfiguration
		previous_configuration.vertical_acceleration_filter_time_seconds = 0.04
		previous_configuration.vertical_response_frequency_hertz = 2.1
		var previous_rig := (load("res://scripts/camera/cockpit_driver_motion_state.gd") as GDScript).new() as RefCounted
		var smoother_rig := (load("res://scripts/camera/cockpit_driver_motion_state.gd") as GDScript).new() as RefCounted
		var smoother_configuration := smoother_modifier.get("configuration") as CockpitCameraConfiguration
		previous_configuration.vertical_positional_filter_time_seconds = 0.04
		previous_rig.set("configuration", previous_configuration)
		smoother_rig.set("configuration", smoother_configuration)
		var elapsed_seconds := 1.0 / float(frames_per_second)
		previous_rig.call("update_positional_correction", Vector3.ZERO, Basis.IDENTITY, 0.0, Vector2.ONE)
		smoother_rig.call("update_positional_correction", Vector3.ZERO, Basis.IDENTITY, 0.0, Vector2.ONE)
		var previous_pitch := 0.0
		var smoother_pitch := 0.0
		var previous_height := 0.0
		var smoother_height := 0.0
		var previous_pitch_change_energy := 0.0
		var smoother_pitch_change_energy := 0.0
		var previous_height_change_energy := 0.0
		var smoother_height_change_energy := 0.0
		for frame_index in range(frames_per_second * 3):
			var elapsed_time := float(frame_index + 1) / float(frames_per_second)
			var bump_shape := 0.0
			if elapsed_time >= 0.25 and elapsed_time < 0.35:
				bump_shape = sin(PI * (elapsed_time - 0.25) / 0.1)
			elif elapsed_time >= 1.0 and elapsed_time < 2.0:
				bump_shape = sin(TAU * 8.0 * elapsed_time)
			var acceleration := Vector3(0.0, 3.0 * bump_shape, 0.0)
			previous_modifier.call("update_inertial_motion", acceleration, elapsed_seconds)
			smoother_modifier.call("update_inertial_motion", acceleration, elapsed_seconds)
			var previous_current_pitch := float(previous_modifier.get("vertical_displacement_meters"))
			var smoother_current_pitch := float(smoother_modifier.get("vertical_displacement_meters"))
			previous_pitch_change_energy += pow(previous_current_pitch - previous_pitch, 2.0)
			smoother_pitch_change_energy += pow(smoother_current_pitch - smoother_pitch, 2.0)
			previous_pitch = previous_current_pitch
			smoother_pitch = smoother_current_pitch
			var anchor_position := Vector3(0.0, 0.012 * bump_shape, -20.0 * elapsed_time)
			previous_rig.call("update_positional_correction", anchor_position, Basis.IDENTITY, elapsed_seconds, Vector2.ONE)
			smoother_rig.call("update_positional_correction", anchor_position, Basis.IDENTITY, elapsed_seconds, Vector2.ONE)
			var previous_current_height := anchor_position.y + float(previous_rig.get("positional_correction_world").y) * previous_configuration.positional_stabilization_strength
			var smoother_current_height := anchor_position.y + float(smoother_rig.get("positional_correction_world").y) * smoother_configuration.positional_stabilization_strength
			previous_height_change_energy += pow(previous_current_height - previous_height, 2.0)
			smoother_height_change_energy += pow(smoother_current_height - smoother_height, 2.0)
			previous_height = previous_current_height
			smoother_height = smoother_current_height
		var angular_change_ratio := sqrt(smoother_pitch_change_energy / previous_pitch_change_energy)
		var positional_change_ratio := sqrt(smoother_height_change_energy / previous_height_change_energy)
		require_condition(angular_change_ratio < 0.85 and positional_change_ratio < 0.95, "Vertical bumps retain abrupt head or camera changes compared with the previous tuning.")
		require_condition(absf(smoother_pitch) < deg_to_rad(0.01) and absf(smoother_height) < 0.00001, "Vertical bump smoothing retains tilt or height offset after the road settles.")
		print("COCKPIT_VERTICAL_BUMP_CHANGE_RATIO=" + str([frames_per_second, angular_change_ratio, positional_change_ratio]))

func validate_positional_stabilization() -> void:
	var ramp_corrections: Array[Vector3] = []
	for frames_per_second in [30, 60, 120]:
		var rig := (load("res://scripts/camera/cockpit_driver_motion_state.gd") as GDScript).new() as RefCounted
		var rig_configuration := CockpitCameraConfiguration.load_from_path("res://data/cameras/formula_one_2030_cockpit_camera.json")
		rig.set("configuration", rig_configuration)
		rig.call("update_positional_correction", Vector3.ZERO, Basis.IDENTITY, 0.0, Vector2.ONE)
		var original_energy := Vector2.ZERO
		var stabilized_energy := Vector2.ZERO
		for frame_index in range(frames_per_second * 2):
			var elapsed_time := float(frame_index + 1) / float(frames_per_second)
			var position := Vector3(0.005 * sin(TAU * 8.0 * elapsed_time), 0.01 * sin(TAU * 8.0 * elapsed_time), -20.0 * elapsed_time)
			rig.call("update_positional_correction", position, Basis.IDENTITY, 1.0 / frames_per_second, Vector2.ONE)
			var correction: Vector3 = rig.get("positional_correction_world")
			require_condition(absf(correction.x) <= 0.010001 and absf(correction.y) <= 0.020001 and absf(correction.z) < 0.000001, "Positional correction exceeds its limits or delays forward travel.")
			if elapsed_time >= 0.5:
				var original_motion := Vector2(position.x, position.y)
				var stabilized_motion := Vector2(position.x + correction.x * rig_configuration.positional_stabilization_strength, position.y + correction.y * rig_configuration.positional_stabilization_strength)
				original_energy += original_motion * original_motion
				stabilized_energy += stabilized_motion * stabilized_motion
		var motion_ratio := Vector2(sqrt(stabilized_energy.x / original_energy.x), sqrt(stabilized_energy.y / original_energy.y))
		require_condition(motion_ratio.x < 0.6 and motion_ratio.y < 0.6, "Positional stabilization does not smooth lateral and vertical bumps.")
		print("COCKPIT_BUMP_POSITIONAL_RESPONSE_RATIO=" + str([frames_per_second, motion_ratio]))
		rig.call("reset_positional_motion")
		rig.call("update_positional_correction", Vector3.ZERO, Basis.IDENTITY, 1.0 / frames_per_second, Vector2.ONE)
		rig.call("update_positional_correction", Vector3(0.2, 0.2, 0.0), Basis.IDENTITY, 1.0 / frames_per_second, Vector2.ONE)
		var correction: Vector3 = rig.get("positional_correction_world")
		require_condition(absf(correction.x + 0.01) < 0.000001 and absf(correction.y + 0.02) < 0.000001, "Large bumps do not obey the configured correction caps.")
		for settle_frame in range(frames_per_second / 2):
			rig.call("update_positional_correction", Vector3(0.2, 0.2, 0.0), Basis.IDENTITY, 1.0 / frames_per_second, Vector2.ONE)
		require_condition((rig.get("positional_correction_world") as Vector3).length() < 0.00001, "Positional correction does not recover the eye anchor after a bump.")
		rig.call("update_positional_correction", Vector3(20.0, 20.0, 0.0), Basis.IDENTITY, 1.0 / frames_per_second, Vector2.ONE)
		require_condition((rig.get("positional_correction_world") as Vector3) == Vector3.ZERO, "Teleport retains a positional correction.")
		rig_configuration.positional_stabilization_strength = 0.0
		rig.call("update_positional_correction", Vector3(20.2, 20.2, 0.0), Basis.IDENTITY, 1.0 / frames_per_second, Vector2.ONE)
		require_condition((rig.get("positional_correction_world") as Vector3) == Vector3.ZERO, "Zero positional sensitivity does not restore exact eye following.")
		rig_configuration.positional_stabilization_strength = 1.0
		rig.call("reset_positional_motion")
		rig.call("update_positional_correction", Vector3.ZERO, Basis.IDENTITY, 0.0, Vector2.ONE)
		for ramp_frame in range(frames_per_second):
			var ramp_time := float(ramp_frame + 1) / float(frames_per_second)
			rig.call("update_positional_correction", Vector3(0.005, 0.01, -20.0) * ramp_time, Basis.IDENTITY, 1.0 / frames_per_second, Vector2.ONE)
		ramp_corrections.append(rig.get("positional_correction_world"))
		rig.call("reset_positional_motion")
		rig.call("update_positional_correction", Vector3.ZERO, Basis.IDENTITY, 0.0, Vector2.ZERO)
		for grade_frame in range(frames_per_second * 2):
			var grade_time := float(grade_frame + 1) / float(frames_per_second)
			var grade_position := Vector3(0.5 * grade_time * grade_time, sin(grade_time) * 2.0, -20.0 * grade_time)
			rig.call("update_positional_correction", grade_position, Basis(Vector3.RIGHT, 0.2 * sin(grade_time)), 1.0 / frames_per_second, Vector2.ZERO)
			require_condition((rig.get("positional_correction_world") as Vector3) == Vector3.ZERO, "Positional filtering introduces lateral or vertical lag during a grade or corner without suspension bumps.")
	for correction in ramp_corrections:
		require_condition(correction.distance_to(ramp_corrections[0]) < 0.000001, "Positional stabilization changes its response to uniform motion with frame rate.")

func validate_shared_motion_sampling() -> void:
	var final_states: Array[Dictionary] = []
	for rendering_frequency in [30, 60, 120, 144]:
		var motion_state := make_motion_modifier()
		var next_render_time := 1.0 / float(rendering_frequency)
		var rendered_sample_count := 0
		for physics_frame_index in range(240):
			var elapsed_time := float(physics_frame_index + 1) / 120.0
			var compression := 12.0 * sin(TAU * 8.0 * elapsed_time)
			var telemetry := {"long_g": 3.0 if elapsed_time < 0.8 else -2.0, "wheel_compressions": PackedFloat64Array([50.0 + compression, 50.0, 50.0 + compression, 50.0])}
			var vehicle_transform := Transform3D(Basis.IDENTITY, Vector3(0.0, 0.005 * sin(TAU * 8.0 * elapsed_time), -20.0 * elapsed_time))
			motion_state.call("update_sample", telemetry, vehicle_transform, Vector3.UP, 1.0 / 120.0, 1.0)
			var sample_count := int(motion_state.get("update_count"))
			while next_render_time <= elapsed_time + 0.000001:
				var interpolation_fraction := clampf((next_render_time - elapsed_time + 1.0 / 120.0) * 120.0, 0.0, 1.0)
				trigger_render_sample(motion_state, interpolation_fraction)
				rendered_sample_count += 1
				next_render_time += 1.0 / float(rendering_frequency)
			require_condition(int(motion_state.get("update_count")) == sample_count, "Rendering advances the shared physics motion state.")
		final_states.append(motion_state.call("get_render_state", 1.0))
		require_condition(rendered_sample_count == rendering_frequency * 2 and int(motion_state.get("update_count")) == 240, "Rendering schedule or physics sample count is inconsistent.")
		var midpoint: Dictionary = motion_state.call("get_render_state", 0.5)
		var previous_state: Dictionary = motion_state.get("previous_render_state")
		var current_state: Dictionary = motion_state.get("current_render_state")
		var expected_position: Vector3 = previous_state["vehicle_transform"].origin.lerp(current_state["vehicle_transform"].origin, 0.5)
		require_condition(midpoint["vehicle_transform"].origin.distance_to(expected_position) < 0.000001, "Camera interpolation does not follow the shared vehicle samples.")
		motion_state.call("update_sample", {}, Transform3D(Basis.IDENTITY, Vector3(100.0, 100.0, 100.0)), Vector3.UP, 1.0 / 120.0, 0.0)
		var teleported_state: Dictionary = motion_state.call("get_render_state", 0.0)
		require_condition(teleported_state["vehicle_transform"].origin == Vector3(100.0, 100.0, 100.0), "Teleport interpolates across the previous vehicle location.")
	for final_state in final_states:
		require_condition(final_state == final_states[0], "Rendering frequency changes the physical camera response.")
	print("COCKPIT_SHARED_PHYSICS_RENDER_FREQUENCIES=" + str([30, 60, 120, 144]))

func trigger_render_sample(motion_state: RefCounted, interpolation_fraction: float) -> void:
	var rendered_state: Dictionary = motion_state.call("get_render_state", interpolation_fraction)
	require_condition((rendered_state["vehicle_transform"] as Transform3D).is_finite() and (rendered_state["angular_displacement"] as Vector2).is_finite(), "Rendering interpolation produces a nonfinite camera pose.")

func validate_camera_chassis_synchronization() -> void:
	var original_vehicle_transform := vehicle.global_transform
	var original_force_strength := configuration.force_response_strength
	var motion_state := head_modifier.get("motion_state") as RefCounted
	var eye_point := driver_controller.call("get_driver_eye_point") as Node3D
	var maximum_forward_anchor_error := 0.0
	var maximum_lateral_anchor_error := 0.0
	head_modifier.set_physics_process(false)
	configuration.force_response_strength = 0.0
	vehicle.global_transform = Transform3D.IDENTITY
	for speed_meters_per_second in [0.0, 30.0, 80.0]:
		for sample_index in range(12):
			motion_state.call("update_sample", {}, vehicle.global_transform, Vector3.UP, 1.0 / 120.0, 0.0)
			vehicle.position += Vector3(0.15, 0.0, -1.0) * speed_meters_per_second / 120.0
			await skeleton.skeleton_updated
			var expected_position := eye_point.global_position + (camera_rig.call("get_viewpoint_elevation_direction") as Vector3) * configuration.viewpoint_elevation_meters
			expected_position += (camera_rig.get("positional_correction_world") as Vector3) * configuration.positional_stabilization_strength
			var anchor_error := camera_rig.global_position - expected_position
			maximum_forward_anchor_error = maxf(maximum_forward_anchor_error, absf(anchor_error.dot(vehicle.global_basis.z)))
			maximum_lateral_anchor_error = maxf(maximum_lateral_anchor_error, absf(anchor_error.dot(vehicle.global_basis.x)))
	require_condition(maximum_forward_anchor_error < 0.00001 and maximum_lateral_anchor_error < 0.00001, "Cockpit camera moves relative to the visible driver when vehicle motion advances between physics sampling and rendering.")
	print("COCKPIT_CHASSIS_ANCHOR_ERROR_METERS=" + str({"forward": maximum_forward_anchor_error, "lateral": maximum_lateral_anchor_error}))
	vehicle.global_transform = original_vehicle_transform
	configuration.force_response_strength = original_force_strength
	motion_state.call("reset_motion")
	head_modifier.set_physics_process(true)
	await settle_frames(5)

func validate_road_reference_and_velocity_alignment() -> void:
	var motion_state := make_motion_modifier()
	var motion_configuration := motion_state.get("configuration") as CockpitCameraConfiguration
	var neutral_telemetry := {"lat_g": 4.0, "vert_g": -4.0, "wheel_compressions": PackedFloat64Array([50.0, 50.0, 50.0, 50.0])}
	motion_state.call("update_sample", neutral_telemetry, Transform3D.IDENTITY, Vector3.UP, 1.0 / 120.0, 0.0)
	var sloped_basis := Basis.from_euler(Vector3(deg_to_rad(20.0), 0.0, deg_to_rad(10.0)))
	motion_state.call("update_sample", neutral_telemetry, Transform3D(sloped_basis, Vector3.ZERO), sloped_basis.y, 1.0 / 120.0, 0.0)
	var initial_reference: Vector2 = motion_state.get("road_reference_angles")
	for physics_frame_index in range(360):
		var elapsed_time := float(physics_frame_index + 1) / 120.0
		var compression := 20.0 * sin(TAU * 4.0 * elapsed_time)
		neutral_telemetry["wheel_compressions"] = PackedFloat64Array([50.0 + compression, 50.0 - compression, 50.0 + compression, 50.0 - compression])
		motion_state.call("update_sample", neutral_telemetry, Transform3D(sloped_basis, -sloped_basis.z * elapsed_time * 20.0), sloped_basis.y, 1.0 / 120.0, 0.0)
	var final_reference: Vector2 = motion_state.get("road_reference_angles")
	var expected_basis: Basis = motion_state.call("make_heading_basis", sloped_basis, sloped_basis.y)
	var expected_flat_basis: Basis = motion_state.call("make_heading_basis", sloped_basis)
	var expected_angles := (expected_flat_basis.inverse() * expected_basis).get_euler()
	require_condition(initial_reference.length() < final_reference.length() * 0.1, "Road reference snaps instantly to a grade or banking change.")
	require_condition(final_reference.distance_to(Vector2(expected_angles.x, expected_angles.z)) < deg_to_rad(0.01), "Road reference does not converge to track grade and banking.")
	require_condition((motion_state.get("angular_displacement") as Vector2) == Vector2.ZERO and float(motion_state.get("vertical_displacement_meters")) == 0.0 and (motion_state.get("positional_correction_world") as Vector3) == Vector3.ZERO, "Flat-road weight transfer or grade produces a false bump.")
	motion_state.call("reset_motion")
	for physics_frame_index in range(120):
		motion_state.call("update_sample", {"linear_velocity": Vector3(5.0, 0.0, -20.0)}, Transform3D.IDENTITY, Vector3.UP, 1.0 / 120.0, 0.0)
	require_condition(float(motion_state.get("velocity_alignment_angle")) == 0.0, "Optional trajectory alignment is enabled by default.")
	motion_configuration.velocity_alignment_strength = 0.4
	for physics_frame_index in range(120):
		motion_state.call("update_sample", {"linear_velocity": Vector3(5.0, 0.0, -20.0)}, Transform3D.IDENTITY, Vector3.UP, 1.0 / 120.0, 0.0)
	var alignment_angle := float(motion_state.get("velocity_alignment_angle"))
	require_condition(alignment_angle < -deg_to_rad(1.0) and absf(alignment_angle) < deg_to_rad(motion_configuration.maximum_velocity_alignment_degrees), "Trajectory alignment has the wrong sign or exceeds its angular limit.")
	for physics_frame_index in range(240):
		motion_state.call("update_sample", {"linear_velocity": Vector3(5.0, 0.0, 20.0)}, Transform3D.IDENTITY, Vector3.UP, 1.0 / 120.0, 0.0)
	require_condition(absf(float(motion_state.get("velocity_alignment_angle"))) < deg_to_rad(0.001), "Reverse travel retains trajectory alignment.")
	print("COCKPIT_ROAD_REFERENCE_AND_OPTIONAL_ALIGNMENT=" + str({"road_angles_degrees": final_reference * 180.0 / PI, "alignment_degrees": rad_to_deg(alignment_angle)}))

func make_collision_box(parent: Node, center: Vector3, size: Vector3) -> StaticBody3D:
	var body := StaticBody3D.new()
	var collision := CollisionShape3D.new()
	var shape := BoxShape3D.new()
	shape.size = size
	collision.shape = shape
	body.add_child(collision)
	parent.add_child(body)
	body.global_position = center
	return body

func validate_road_bump_confirmation() -> void:
	var fixture := Node3D.new()
	root.add_child(fixture)
	make_collision_box(fixture, Vector3(100.0, -0.2, 0.0), Vector3(10.0, 0.4, 10.0))
	var telemetry_vehicle := TelemetryVehicle.new()
	fixture.add_child(telemetry_vehicle)
	telemetry_vehicle.position.x = 100.0
	telemetry_vehicle.raycasts.resize(12)
	for wheel_index in range(4):
		var raycast := RayCast3D.new()
		raycast.position = Vector3(-0.6 if wheel_index % 2 == 0 else 0.6, 0.5, -1.0 if wheel_index < 2 else 1.0)
		raycast.target_position = Vector3(0.0, -1.0, 0.0)
		telemetry_vehicle.add_child(raycast)
		telemetry_vehicle.raycasts[wheel_index * 3 + 1] = raycast
	await physics_frame
	var motion_state := make_motion_modifier()
	motion_state.set("vehicle", telemetry_vehicle)
	for physics_frame_index in range(60):
		for raycast_index in [1, 4, 7, 10]:
			(telemetry_vehicle.raycasts[raycast_index] as RayCast3D).force_raycast_update()
		var compression := 20.0 * sin(TAU * 4.0 * float(physics_frame_index) / 120.0)
		telemetry_vehicle.wheel_compressions = PackedFloat64Array([50.0 + compression, 50.0 - compression, 50.0 + compression, 50.0 - compression])
		motion_state.call("update_from_vehicle", 1.0 / 120.0)
	require_condition(bool(motion_state.get("has_road_bump_confirmation")) and float(motion_state.get("road_bump_confirmation")) < 0.001, "Flat collision contacts falsely confirm a suspension bump.")
	require_condition((motion_state.get("angular_displacement") as Vector2) == Vector2.ZERO, "Fast suspension load transfer on flat collision geometry moves the head.")
	make_collision_box(fixture, Vector3(99.4, 0.015, -1.0), Vector3(0.25, 0.03, 0.25))
	await physics_frame
	var peak_roll := 0.0
	for physics_frame_index in range(60):
		for raycast_index in [1, 4, 7, 10]:
			(telemetry_vehicle.raycasts[raycast_index] as RayCast3D).force_raycast_update()
		telemetry_vehicle.wheel_compressions = PackedFloat64Array([50.0 + 20.0 * sin(TAU * 8.0 * float(physics_frame_index) / 120.0), 50.0, 50.0, 50.0])
		motion_state.call("update_from_vehicle", 1.0 / 120.0)
		peak_roll = maxf(peak_roll, absf(float(motion_state.get("angular_displacement").y)))
	require_condition(float(motion_state.get("road_bump_confirmation")) > 0.9 and peak_roll > deg_to_rad(0.005), "A real wheel contact height discontinuity does not retain bump feedback.")
	print("COCKPIT_CONTACT_CONFIRMED_BUMP_ROLL_DEGREES=" + str(rad_to_deg(peak_roll)))
	fixture.free()

func validate_motion_preferences() -> void:
	var panel := camera_rig.get("preferences_panel") as CanvasLayer
	require_condition(panel != null and (panel.get("sliders") as Dictionary).size() == 6, "Cockpit preferences do not expose all six motion controls.")
	require_condition(camera_rig.get("configuration") == head_modifier.get("configuration"), "Camera and pilot preferences use different configuration instances.")
	camera_rig.call("set_view_active", true)
	var key_event := InputEventKey.new()
	key_event.physical_keycode = KEY_F9
	key_event.pressed = true
	camera_rig.call("_unhandled_input", key_event)
	require_condition(panel.visible, "F9 does not show cockpit motion preferences.")
	var sliders: Dictionary = panel.get("sliders")
	(sliders["longitudinal_force_response_strength"] as HSlider).value = 0.0
	require_condition(configuration.longitudinal_force_response_strength == 0.0, "Motion slider does not update the shared pilot configuration.")
	configuration.apply_motion_preferences({"velocity_alignment_strength": 5.0, "vertical_bump_response_strength": NAN, "maximum_pitch_degrees": 80.0})
	require_condition(configuration.velocity_alignment_strength == 1.0 and configuration.vertical_bump_response_strength == 1.0 and configuration.maximum_pitch_degrees == 6.0, "Preferences allow invalid values or change unrelated tuning.")
	panel.call("reset_preferences")
	require_condition(configuration.longitudinal_force_response_strength == 1.0 and configuration.velocity_alignment_strength == 0.0, "Restoring preferences does not recover the initial tuning.")
	camera_rig.call("set_view_active", false)
	require_condition(not panel.visible, "Leaving cockpit retains the preferences panel.")
	await process_frame
