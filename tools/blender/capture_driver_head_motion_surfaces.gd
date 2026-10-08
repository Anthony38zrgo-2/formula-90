extends SceneTree

const HEAD_MOTION_SCRIPT = preload("res://scripts/vehicle/driver_head_motion_modifier.gd")
const MOTION_STATE_SCRIPT = preload("res://scripts/camera/cockpit_driver_motion_state.gd")
const CAMERA_CONFIGURATION_SCRIPT = preload("res://scripts/camera/cockpit_camera_configuration.gd")

func _init() -> void:
	call_deferred("capture_head_surfaces")

func capture_head_surfaces() -> void:
	var document := GLTFDocument.new()
	var document_state := GLTFState.new()
	var command_arguments := OS.get_cmdline_user_args()
	if command_arguments.size() != 4:
		printerr("Head capture requires repository, driver model, camera configuration and output paths.")
		quit(1)
		return
	var project_directory := command_arguments[0]
	var model_path := command_arguments[1]
	var camera_configuration_path := command_arguments[2]
	var output_path := command_arguments[3]
	var validation_output: Array = []
	var validation_result := OS.execute("python", [project_directory.path_join("tools/common/output_policy.py"), "--repo", project_directory, "--mode", "preview", "--output", output_path], validation_output)
	if validation_result != 0:
		printerr("Head surface output path was rejected.")
		quit(1)
		return
	if document.append_from_file(model_path, document_state) != OK:
		quit(1)
		return
	var driver_instance := document.generate_scene(document_state)
	var vehicle := Node3D.new()
	root.add_child(vehicle)
	vehicle.add_child(driver_instance)
	driver_instance.position = Vector3(0.0, -0.011, -0.24)
	driver_instance.rotation.y = PI
	await process_frame
	var skeleton := driver_instance.find_children("*", "Skeleton3D", true, false)[0] as Skeleton3D
	var head_mesh := driver_instance.find_child("DriverHeadAndNeck", true, false) as MeshInstance3D
	var configuration = CAMERA_CONFIGURATION_SCRIPT.load_from_path(camera_configuration_path)
	var modifier = HEAD_MOTION_SCRIPT.new()
	modifier.vehicle = vehicle
	modifier.configuration = configuration
	modifier.motion_state = MOTION_STATE_SCRIPT.new()
	modifier.head_bone_index = skeleton.find_bone("mixamorig_Head")
	modifier.neck_bone_index = skeleton.find_bone("mixamorig_Neck")
	if modifier.head_bone_index < 0:
		modifier.head_bone_index = skeleton.find_bone("mixamorig:Head")
		modifier.neck_bone_index = skeleton.find_bone("mixamorig:Neck")
	var neutral_poses: Array[Transform3D] = []
	for bone_index in skeleton.get_bone_count():
		neutral_poses.append(skeleton.get_bone_pose(bone_index))
	var sample_settings: Array[Dictionary] = []
	for pitch_degrees in [-6.0, -3.0, 0.0, 3.0, 6.0]:
		for roll_degrees in [-3.0, 0.0, 3.0]:
			sample_settings.append({"pitch_degrees": pitch_degrees, "roll_degrees": roll_degrees, "road_pitch_degrees": 0.0, "road_roll_degrees": 0.0, "category": "configured_nod_and_roll"})
	for road_pitch_degrees in [-45.0, -30.0, -15.0, 0.0, 15.0, 30.0, 45.0]:
		for road_roll_degrees in [-45.0, -30.0, -15.0, 0.0, 15.0, 30.0, 45.0]:
			sample_settings.append({"pitch_degrees": 0.0, "roll_degrees": 0.0, "road_pitch_degrees": road_pitch_degrees, "road_roll_degrees": road_roll_degrees, "category": "horizon_correction_extremes"})
	var samples: Array[Dictionary] = []
	for setting in sample_settings:
		for bone_index in skeleton.get_bone_count():
			skeleton.set_bone_pose(bone_index, neutral_poses[bone_index])
		modifier.angular_displacement = Vector2(deg_to_rad(setting.pitch_degrees), deg_to_rad(setting.roll_degrees))
		modifier.apply_head_and_neck_pose(skeleton, {"road_reference_angles": Vector2(deg_to_rad(setting.road_pitch_degrees), deg_to_rad(setting.road_roll_degrees)), "velocity_alignment_angle": 0.0})
		var vertices: Array = []
		var triangles: Array = []
		for surface_index in head_mesh.mesh.get_surface_count():
			var surface_arrays := head_mesh.mesh.surface_get_arrays(surface_index)
			var source_vertices: PackedVector3Array = surface_arrays[Mesh.ARRAY_VERTEX]
			var bone_bind_indices: PackedInt32Array = surface_arrays[Mesh.ARRAY_BONES]
			var bone_weights: PackedFloat32Array = surface_arrays[Mesh.ARRAY_WEIGHTS]
			var triangle_indices: PackedInt32Array = surface_arrays[Mesh.ARRAY_INDEX]
			var influence_count := int(bone_weights.size() / source_vertices.size())
			var vertex_offset := vertices.size()
			for vertex_index in source_vertices.size():
				var skinned_position := Vector3.ZERO
				for influence_index in influence_count:
					var weight_index := vertex_index * influence_count + influence_index
					var weight := bone_weights[weight_index]
					if weight <= 0.0:
						continue
					var bind_index := bone_bind_indices[weight_index]
					var bone_name := head_mesh.skin.get_bind_name(bind_index)
					var bone_index := skeleton.find_bone(bone_name) if not bone_name.is_empty() else head_mesh.skin.get_bind_bone(bind_index)
					var skin_transform := skeleton.get_bone_global_pose(bone_index) * head_mesh.skin.get_bind_pose(bind_index)
					skinned_position += skin_transform * source_vertices[vertex_index] * weight
				var chassis_position := skeleton.global_transform * skinned_position
				vertices.append([chassis_position.x, -chassis_position.z, chassis_position.y])
			for triangle_index in range(0, triangle_indices.size(), 3):
				triangles.append([triangle_indices[triangle_index] + vertex_offset, triangle_indices[triangle_index + 1] + vertex_offset, triangle_indices[triangle_index + 2] + vertex_offset])
		var sample = setting.duplicate()
		sample["vertices"] = vertices
		sample["triangles"] = triangles
		samples.append(sample)
	var output_file := FileAccess.open(output_path, FileAccess.WRITE)
	output_file.store_string(JSON.stringify({"capture_script_sha256": FileAccess.get_sha256(get_script().resource_path), "driver_sha256": FileAccess.get_sha256(model_path), "modifier_sha256": FileAccess.get_sha256("res://scripts/vehicle/driver_head_motion_modifier.gd"), "configuration_sha256": FileAccess.get_sha256(camera_configuration_path), "samples": samples}))
	output_file.close()
	modifier.free()
	vehicle.queue_free()
	await process_frame
	print("HEAD_MOTION_CAPTURE_SAMPLES=" + str(samples.size()))
	quit(0)
