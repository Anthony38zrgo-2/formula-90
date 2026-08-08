extends Node

var enabled := true
var vehicle : Vehicle

var _file : FileAccess
var _buffer := PackedStringArray()
var _prev_velocity := Vector3.ZERO
var _prev_velocity_time := 0
var _prev_sample_time := 0
var _file_opened := false
var _search_timer := 0.0

const LOG_MS := 50
const BUFFER_SIZE := 100
const VEHICLE_SEARCH_INTERVAL := 1.0

func _ready():
	_prev_sample_time = Time.get_ticks_msec()
	_prev_velocity_time = _prev_sample_time

func _physics_process(delta):
	if not enabled:
		return

	if not vehicle:
		_search_timer += delta
		if _search_timer >= VEHICLE_SEARCH_INTERVAL:
			_search_timer = 0.0
			_try_find_vehicle()
		return
	_search_timer = 0.0

	var current_velocity = vehicle.linear_velocity

	if not _file_opened:
		_open_file()
		_prev_velocity = current_velocity
		_prev_velocity_time = Time.get_ticks_msec()

	var now = Time.get_ticks_msec()
	if now - _prev_sample_time < LOG_MS:
		return
	_prev_sample_time = now

	var line = _format_line(now, current_velocity)
	_buffer.append(line)
	if _buffer.size() >= BUFFER_SIZE:
		_flush()

func _exit_tree():
	if _file_opened:
		_flush()
		_file.close()
		_file_opened = false
	_file = null

func _try_find_vehicle():
	var nodes = get_tree().root.find_children("*", "Vehicle", true, false)
	if nodes.size() > 0:
		vehicle = nodes[0] as Vehicle
		_prev_velocity_time = 0

func _open_file():
	var dir = "res://telemetry/"
	DirAccess.make_dir_recursive_absolute(dir)

	var dt = Time.get_datetime_dict_from_system()
	var msec = Time.get_ticks_msec() % 1000
	var name = "telemetry_%04d%02d%02d_%02d%02d%02d_%03d.csv" % [
		dt.year, dt.month, dt.day,
		dt.hour, dt.minute, dt.second, msec
	]
	var path = dir + name
	_file = FileAccess.open(path, FileAccess.WRITE)
	if not _file:
		push_error("[TelemetryManager] Cannot open file: ", path)
		return
	_file_opened = true
	_file.store_csv_line(PackedStringArray([
		"Time_ms", "Speed_kmh", "RPM", "Gear",
		"Throttle", "Brake", "Steering",
		"Lat_G", "Long_G",
		"FL_Comp", "FR_Comp", "RL_Comp", "RR_Comp",
		"Front_Slip", "Rear_Slip"
	]))


func _format_line(now_msec: int, current_velocity: Vector3) -> String:
	var speed_kmh = abs(vehicle.speed) * 3.6
	var rpm = vehicle.motor_rpm
	var gear = vehicle.current_gear
	var throttle = vehicle.throttle_amount
	var brake_amt = vehicle.brake_amount
	var steering = vehicle.steering_input

	var lat_g := 0.0
	var long_g := 0.0
	if _prev_velocity_time > 0:
		var elapsed = max((now_msec - _prev_velocity_time) / 1000.0, 0.001)
		var accel = (current_velocity - _prev_velocity) / elapsed
		var local_accel = vehicle.global_transform.basis.inverse() * accel
		lat_g = local_accel.x / 9.81
		long_g = -local_accel.z / 9.81
	_prev_velocity = current_velocity
	_prev_velocity_time = now_msec

	var fl_comp = vehicle.front_axle.suspension_compression_left if vehicle.front_axle else 0.0
	var fr_comp = vehicle.front_axle.suspension_compression_right if vehicle.front_axle else 0.0
	var rl_comp = vehicle.rear_axle.suspension_compression_left if vehicle.rear_axle else 0.0
	var rr_comp = vehicle.rear_axle.suspension_compression_right if vehicle.rear_axle else 0.0

	var front_slip = vehicle.front_axle.get_max_wheel_slip_y() if vehicle.front_axle else 0.0
	var rear_slip = vehicle.rear_axle.get_max_wheel_slip_y() if vehicle.rear_axle else 0.0

	return "%d,%.1f,%d,%d,%.3f,%.3f,%.3f,%.3f,%.3f,%.1f,%.1f,%.1f,%.1f,%.3f,%.3f" % [
		now_msec, speed_kmh, rpm, gear,
		throttle, brake_amt, steering,
		lat_g, long_g,
		fl_comp, fr_comp, rl_comp, rr_comp,
		front_slip, rear_slip
	]

func _flush():
	if _file and _buffer.size() > 0:
		_file.store_string("\n".join(_buffer) + "\n")
		_buffer.clear()
