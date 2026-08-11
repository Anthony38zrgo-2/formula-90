extends SceneTree

const SCENE := preload("res://scenes/vehicles/f1_90s_canonical_1997/jordan_191_candidate_validation.tscn")

func _init() -> void:
	var root := SCENE.instantiate()
	var vehicle := root.get_node_or_null("CandidateVehicle")
	var failures: Array[String] = []
	if vehicle == null:
		failures.append("CandidateVehicle missing")
	else:
		_check_axle(vehicle, "WheelFrontLeft", "WheelFrontRight", failures)
		_check_axle(vehicle, "WheelRearLeft", "WheelRearRight", failures)
		if vehicle.get_node_or_null("ChassisVisual") == null:
			failures.append("ChassisVisual missing")
	root.queue_free()
	if failures.is_empty():
		print("PASS: Jordan 191 candidate validation scene contract")
		quit(0)
	else:
		push_error("; ".join(failures))
		quit(1)

func _check_axle(vehicle: Node3D, left_name: String, right_name: String, failures: Array[String]) -> void:
	var left := vehicle.get_node_or_null(left_name) as Node3D
	var right := vehicle.get_node_or_null(right_name) as Node3D
	if left == null or right == null:
		failures.append("%s axle nodes missing" % left_name)
		return
	if not is_zero_approx(left.position.x + right.position.x) or not is_equal_approx(left.position.y, right.position.y) or not is_equal_approx(left.position.z, right.position.z):
		failures.append("%s positions are not symmetric" % left_name)
	var left_visual := left.get_node_or_null("Pivot/Orientation/Visual")
	var right_visual := right.get_node_or_null("Pivot/Orientation/Visual")
	if left_visual == null or right_visual == null or left_visual.scene_file_path != right_visual.scene_file_path:
		failures.append("%s wheel scenes differ" % left_name)
	var right_orientation := right.get_node_or_null("Pivot/Orientation") as Node3D
	if right_orientation == null or not is_equal_approx(absf(right_orientation.rotation.y), PI):
		failures.append("%s right orientation is not PI around Y" % left_name)
