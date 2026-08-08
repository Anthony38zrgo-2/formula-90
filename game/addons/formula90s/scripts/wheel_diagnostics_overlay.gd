extends Label

## Lightweight runtime diagnostics for GEVP wheel stability.
## Toggle with the existing ShowDebug input. It is intentionally read-only.

@export var vehicle_path: NodePath
@export var toggle_action := "ShowDebug"

var _previous_lengths: Dictionary = {}

func _ready() -> void:
	visible = false

func _process(_delta: float) -> void:
	if InputMap.has_action(toggle_action) and Input.is_action_just_pressed(toggle_action):
		visible = not visible

	if not visible:
		return

	var vehicle := get_node_or_null(vehicle_path)
	if not vehicle:
		text = "WHEEL DEBUG: vehicle missing"
		return

	var lines: Array[String] = ["WHEEL DEBUG  contact / compression / dLen / slipX / slipY"]
	_append_wheel(lines, vehicle, "FL", "WheelFrontLeft")
	_append_wheel(lines, vehicle, "FR", "WheelFrontRight")
	_append_wheel(lines, vehicle, "RL", "WheelRearLeft")
	_append_wheel(lines, vehicle, "RR", "WheelRearRight")
	text = "\n".join(lines)

func _append_wheel(lines: Array[String], vehicle: Node, short_name: String, node_name: String) -> void:
	var wheel := vehicle.get_node_or_null(NodePath(node_name))
	if not wheel:
		lines.append("%s missing" % short_name)
		return

	var spring_length := float(wheel.get("spring_length"))
	var current_length := float(wheel.get("spring_current_length"))
	var compression_mm := (spring_length - current_length) * 1000.0
	var previous_length := float(_previous_lengths.get(node_name, current_length))
	var delta_length_mm := (current_length - previous_length) * 1000.0
	_previous_lengths[node_name] = current_length

	var slip: Vector2 = wheel.get("slip_vector")
	var contact := "C" if wheel.is_colliding() else "-"
	lines.append(
		"%s %s  %.1fmm  d%.2f  x%.3f  y%.3f" % [
			short_name,
			contact,
			compression_mm,
			delta_length_mm,
			slip.x,
			slip.y
		]
	)
