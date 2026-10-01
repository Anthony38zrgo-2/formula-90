extends SceneTree

class SteeringTelemetryVehicle extends Node3D:
	var steering_amount: float = 0.0
	func get_true_steering_amount() -> float:
		return steering_amount

var driver_controller: Node
var captured_samples: Array[Dictionary] = []
var capture_next_modification := false
var current_steering_degrees := 0.0

func _init() -> void:
	call_deferred("capture_seated_surfaces")

func capture_seated_surfaces() -> void:
	var output_path := OS.get_cmdline_user_args()[0]
	var project_directory := ProjectSettings.globalize_path("res://").trim_suffix("/").get_base_dir()
	var validation_output: Array = []
	var validation_result := OS.execute("python", [project_directory.path_join("tools/common/output_policy.py"), "--repo", project_directory, "--mode", "preview", "--output", output_path], validation_output)
	if validation_result != 0:
		printerr("Driver surface output path was rejected.")
		quit(1)
		return
	var vehicle_scene := load("res://scenes/vehicles/f1_2030_v10/f1_2030_v10_rust.tscn") as PackedScene
	var instance := vehicle_scene.instantiate()
	root.add_child(instance)
	var vehicle := instance.get_node("VehicleRigidBody")
	vehicle.set("freeze", true)
	instance.get_node("F12030V10InputController").set_process(false)
	driver_controller = vehicle.get_node("DriverVisualController")
	var telemetry := SteeringTelemetryVehicle.new()
	root.add_child(telemetry)
	var steering_controller := vehicle.get_node("SteeringWheelVisualController")
	steering_controller.set("vehicle", telemetry)
	var modifier := driver_controller.get("arm_modifier") as SkeletonModifier3D
	modifier.modification_processed.connect(capture_current_surface)
	for steering_degrees in [0.0, 90.0, 120.0, 180.0, -90.0, -120.0, -180.0]:
		current_steering_degrees = steering_degrees
		telemetry.steering_amount = steering_degrees / 180.0
		steering_controller.call("update_steering_wheel_pose")
		for frame_index in range(40):
			await process_frame
		capture_next_modification = true
		while capture_next_modification:
			await process_frame
	var output_file := FileAccess.open(output_path, FileAccess.WRITE)
	if output_file == null:
		quit(1)
		return
	output_file.store_string(JSON.stringify({"samples": captured_samples}))
	output_file.close()
	print("DRIVER_SURFACE_SAMPLES=" + str(captured_samples.size()))
	instance.queue_free()
	telemetry.queue_free()
	await process_frame
	quit(0)

func capture_current_surface() -> void:
	if not capture_next_modification:
		return
	capture_next_modification = false
	var skeleton := driver_controller.get("driver_skeleton") as Skeleton3D
	var driver_instance := driver_controller.get("driver_instance") as Node3D
	var chassis := driver_controller.get("chassis_visual") as Node3D
	var skeleton_to_chassis := chassis.global_transform.affine_inverse() * skeleton.global_transform
	var vertices: Array = []
	var triangles: Array = []
	var dominant_bones: Array[String] = []
	for descendant in driver_instance.find_children("*", "MeshInstance3D", true, false):
		var driver_body := descendant as MeshInstance3D
		if driver_body.skin == null:
			continue
		for surface_index in range(driver_body.mesh.get_surface_count()):
			var surface_arrays := driver_body.mesh.surface_get_arrays(surface_index)
			var source_vertices: PackedVector3Array = surface_arrays[Mesh.ARRAY_VERTEX]
			var bone_bind_indices: PackedInt32Array = surface_arrays[Mesh.ARRAY_BONES]
			var bone_weights: PackedFloat32Array = surface_arrays[Mesh.ARRAY_WEIGHTS]
			var triangle_indices: PackedInt32Array = surface_arrays[Mesh.ARRAY_INDEX]
			var influence_count := int(bone_weights.size() / source_vertices.size())
			var vertex_offset := vertices.size()
			for vertex_index in range(source_vertices.size()):
				var skinned_position := Vector3.ZERO
				var maximum_weight := -1.0
				var dominant_bone_name := ""
				for influence_index in range(influence_count):
					var weight_index := vertex_index * influence_count + influence_index
					var weight := bone_weights[weight_index]
					if weight <= 0.0:
						continue
					var bind_index := bone_bind_indices[weight_index]
					var bone_name := driver_body.skin.get_bind_name(bind_index)
					var bone_index := skeleton.find_bone(bone_name) if not bone_name.is_empty() else driver_body.skin.get_bind_bone(bind_index)
					var skin_transform := skeleton.get_bone_global_pose(bone_index) * driver_body.skin.get_bind_pose(bind_index)
					skinned_position += skin_transform * source_vertices[vertex_index] * weight
					if weight > maximum_weight:
						maximum_weight = weight
						dominant_bone_name = skeleton.get_bone_name(bone_index)
				var chassis_position := skeleton_to_chassis * skinned_position
				vertices.append([chassis_position.x, -chassis_position.z, chassis_position.y])
				dominant_bones.append(dominant_bone_name)
			for triangle_index in range(0, triangle_indices.size(), 3):
				triangles.append([triangle_indices[triangle_index] + vertex_offset, triangle_indices[triangle_index + 1] + vertex_offset, triangle_indices[triangle_index + 2] + vertex_offset])
	captured_samples.append({"steering_degrees": current_steering_degrees, "vertices": vertices, "triangles": triangles, "dominant_bones": dominant_bones})
