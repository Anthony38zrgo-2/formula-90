extends SceneTree

class SteeringTelemetryVehicle extends Node3D:
	var effective_steering_amount: float = 0.0

	func get_true_steering_amount() -> float:
		return effective_steering_amount

var failures: Array[String] = []
var source_directory: String

func _init() -> void:
	call_deferred("run_validation")

func verify(condition: bool, message: String) -> void:
	if not condition:
		failures.append(message)
		printerr(message)

func run_validation() -> void:
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--source-directory="):
			source_directory = argument.trim_prefix("--source-directory=")
	if source_directory.is_empty():
		printerr("Missing --source-directory argument.")
		quit(1)
		return
	var controller_script := load(source_directory.path_join("scripts/vehicle/steering_wheel_visual_controller.gd")) as Script
	if controller_script == null or not controller_script.can_instantiate():
		printerr("Steering wheel controller cannot load.")
		quit(1)
		return
	validate_model(controller_script, "assets/models/vehicles/f1-2030/f1_2030_v10_chassis.glb")
	validate_model(controller_script, "assets/models/vehicles/f1-2030/liveries/mp4_6_senna_1/f1_2030_v10_chassis.glb")
	for scene_filename in ["f1_2030_v10_rust.tscn", "f1_2030_v10_rust_mp4_6_senna_1.tscn"]:
		var scene_source := FileAccess.get_file_as_string(source_directory.path_join("scenes/vehicles/f1_2030_v10/" + scene_filename))
		verify(scene_source.contains('path="res://scripts/vehicle/steering_wheel_visual_controller.gd"'), scene_filename + ": controller resource missing")
		verify(scene_source.contains('chassis_visual = NodePath("../ChassisVisual")'), scene_filename + ": chassis wiring missing")
		verify(scene_source.contains("total_rotation_degrees = 360.0"), scene_filename + ": rotation range missing")
	print("STEERING_WHEEL_VALIDATION_FAILURES=" + str(failures.size()))
	quit(0 if failures.is_empty() else 1)

func validate_model(controller_script: Script, model_relative_path: String) -> void:
	var model_document := GLTFDocument.new()
	var model_state := GLTFState.new()
	var import_error := model_document.append_from_file(source_directory.path_join(model_relative_path), model_state)
	verify(import_error == OK, model_relative_path + ": model import failed")
	if import_error != OK:
		return
	var chassis_visual := model_document.generate_scene(model_state)
	var vehicle := SteeringTelemetryVehicle.new()
	vehicle.transform = Transform3D(Basis(Vector3.UP, 0.6), Vector3(3.0, 2.0, -4.0))
	root.add_child(vehicle)
	vehicle.add_child(chassis_visual)
	var steering_wheel := chassis_visual.find_child("GEO_CHASSIS_STEER", true, false) as MeshInstance3D
	var steering_column := chassis_visual.find_child("GEO_CHASSIS_STEERCOLUM", true, false) as MeshInstance3D
	verify(steering_wheel != null and steering_column != null, model_relative_path + ": steering meshes missing")
	if steering_wheel == null or steering_column == null:
		vehicle.free()
		return
	var neutral_wheel_transform := steering_wheel.global_transform
	var neutral_column_transform := steering_column.global_transform
	var steering_wheel_components: Array[MeshInstance3D] = []
	var neutral_component_transforms: Array[Transform3D] = []
	var component_transforms_relative_to_wheel: Array[Transform3D] = []
	for component_name in ["GEO_CHASSIS_SHIFTERBRAK", "GEO_CHASSIS_SHIFTERTHRO", "GEO_CHASSIS_SCREEN"]:
		var steering_wheel_component := chassis_visual.find_child(component_name, true, false) as MeshInstance3D
		verify(steering_wheel_component != null, model_relative_path + ": missing steering wheel component " + component_name)
		if steering_wheel_component == null:
			vehicle.free()
			return
		steering_wheel_components.append(steering_wheel_component)
		neutral_component_transforms.append(steering_wheel_component.global_transform)
		component_transforms_relative_to_wheel.append(neutral_wheel_transform.affine_inverse() * steering_wheel_component.global_transform)
	var controller := Node.new()
	controller.set_script(controller_script)
	controller.set("vehicle", vehicle)
	controller.set("chassis_visual", chassis_visual)
	vehicle.add_child(controller)
	var steering_wheel_pivot := controller.get("steering_wheel_pivot") as Node3D
	verify(steering_wheel_pivot != null, model_relative_path + ": steering pivot missing")
	if steering_wheel_pivot == null:
		vehicle.free()
		return
	verify(steering_wheel.global_transform.is_equal_approx(neutral_wheel_transform), model_relative_path + ": neutral geometry moved")
	for component_index in range(steering_wheel_components.size()):
		verify(steering_wheel_components[component_index].global_transform.is_equal_approx(neutral_component_transforms[component_index]), model_relative_path + ": neutral steering wheel component moved")
	var column_bounds: AABB = (chassis_visual.global_transform.affine_inverse() * steering_column.global_transform) * steering_column.get_aabb()
	verify(absf(steering_wheel_pivot.position.x - column_bounds.get_center().x) < 0.00001, model_relative_path + ": pivot outside column horizontal axis")
	verify(absf(steering_wheel_pivot.position.y - column_bounds.get_center().y) < 0.00001, model_relative_path + ": pivot outside column vertical axis")
	var neutral_pivot_position := steering_wheel_pivot.position
	for effective_amount in [0.5, -0.5, 1.0, -1.0, 2.0, -2.0, 0.0, NAN]:
		vehicle.effective_steering_amount = effective_amount
		controller.call("_process", 1.0 / 60.0)
		var expected_amount: float = clampf(effective_amount, -1.0, 1.0) if is_finite(effective_amount) else 0.0
		verify(steering_wheel_pivot.basis.is_equal_approx(Basis(Vector3.BACK, expected_amount * PI)), model_relative_path + ": unexpected steering rotation for " + str(effective_amount))
		verify(steering_wheel_pivot.position.is_equal_approx(neutral_pivot_position), model_relative_path + ": pivot drifted")
		verify(steering_column.global_transform.is_equal_approx(neutral_column_transform), model_relative_path + ": fixed column moved")
		for component_index in range(steering_wheel_components.size()):
			var current_relative_transform := steering_wheel.global_transform.affine_inverse() * steering_wheel_components[component_index].global_transform
			verify(current_relative_transform.is_equal_approx(component_transforms_relative_to_wheel[component_index]), model_relative_path + ": steering wheel component detached from steering wheel")
		if effective_amount == 0.5:
			verify((steering_wheel_pivot.basis * Vector3.UP).is_equal_approx(Vector3.LEFT), model_relative_path + ": left steering rotates clockwise")
			for component_index in range(steering_wheel_components.size()):
				verify(not steering_wheel_components[component_index].global_transform.is_equal_approx(neutral_component_transforms[component_index]), model_relative_path + ": steering wheel component stayed fixed to chassis")
	verify(steering_wheel.global_transform.is_equal_approx(neutral_wheel_transform), model_relative_path + ": return to neutral did not restore geometry")
	for component_index in range(steering_wheel_components.size()):
		verify(steering_wheel_components[component_index].global_transform.is_equal_approx(neutral_component_transforms[component_index]), model_relative_path + ": steering wheel component failed to return to neutral")
	print("VALIDATED_STEERING_MODEL=" + model_relative_path)
	vehicle.free()
