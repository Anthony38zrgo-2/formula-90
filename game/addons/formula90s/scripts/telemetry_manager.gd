extends Node

var enabled := true
var vehicle                      # untyped: accepts both the GEVP `Vehicle` and the Rust `F194RustVehicle` node
var _is_rust := false            # true when `vehicle` is the Rust F194RustVehicle (facade) route

var _file : FileAccess
var _buffer := PackedStringArray()
var _prev_velocity := Vector3.ZERO
var _prev_velocity_time := 0
var _prev_sample_time := 0
var _file_opened := false
var _search_timer := 0.0
var _session_id := ""
var _session_timestamp_utc := ""
var _setup_json := ""

const LOG_MS := 50
const BUFFER_SIZE := 100
const VEHICLE_SEARCH_INTERVAL := 1.0
const CSV_COLUMNS := [
    "Time_ms", "Speed_kmh", "RPM", "Gear",
    "Throttle", "Brake", "Steering",
    "Lat_G", "Long_G",
    "FL_Comp", "FR_Comp", "RL_Comp", "RR_Comp",
    "Front_Slip", "Rear_Slip",
    "Session_Id", "Session_Timestamp_UTC", "Physics_Hz",
    "Test_Id", "Track_Scene", "Vehicle_Node_Path", "Vehicle_Scene",
    "Vehicle_Script", "Setup_Schema_Version", "Setup_JSON", "TC_Active"
]

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
    # Rust route (facade / F194RustVehicle) takes precedence; GEVP `Vehicle` is the legacy fallback.
    var rust_nodes = get_tree().root.find_children("*", "F194RustVehicle", true, false)
    if rust_nodes.size() > 0:
        vehicle = rust_nodes[0]
        _is_rust = true
        _prev_velocity_time = 0
        return
    var gevp_nodes = get_tree().root.find_children("*", "Vehicle", true, false)
    if gevp_nodes.size() > 0:
        vehicle = gevp_nodes[0]
        _is_rust = false
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
    _session_id = name.trim_suffix(".csv")
    _session_timestamp_utc = Time.get_datetime_string_from_system(true)
    _setup_json = JSON.stringify(_build_setup_snapshot(name))
    _file.store_csv_line(PackedStringArray(CSV_COLUMNS))


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

    var fl_comp := 0.0
    var fr_comp := 0.0
    var rl_comp := 0.0
    var rr_comp := 0.0
    var front_slip := 0.0
    var rear_slip := 0.0

    if _is_rust:
        var comp = vehicle.get_wheel_compressions()
        if comp.size() >= 4:
            fl_comp = comp[0]; fr_comp = comp[1]; rl_comp = comp[2]; rr_comp = comp[3]
        var slips = vehicle.get_wheel_slips()
        if slips.size() >= 4:
            front_slip = maxf(absf(slips[0]), absf(slips[1]))
            rear_slip = maxf(absf(slips[2]), absf(slips[3]))
    else:
        fl_comp = vehicle.front_axle.suspension_compression_left if vehicle.front_axle else 0.0
        fr_comp = vehicle.front_axle.suspension_compression_right if vehicle.front_axle else 0.0
        rl_comp = vehicle.rear_axle.suspension_compression_left if vehicle.rear_axle else 0.0
        rr_comp = vehicle.rear_axle.suspension_compression_right if vehicle.rear_axle else 0.0
        front_slip = vehicle.front_axle.get_max_wheel_slip_y() if vehicle.front_axle else 0.0
        rear_slip = vehicle.rear_axle.get_max_wheel_slip_y() if vehicle.rear_axle else 0.0

    return "%d,%.1f,%d,%d,%.3f,%.3f,%.3f,%.3f,%.3f,%.1f,%.1f,%.1f,%.1f,%.3f,%.3f,%s,%s,%d,%s,%s,%s,%s,%s,%d,%s,%d" % [
        now_msec, speed_kmh, rpm, gear,
        throttle, brake_amt, steering,
        lat_g, long_g,
        fl_comp, fr_comp, rl_comp, rr_comp,
        front_slip, rear_slip,
        _csv_escape(_session_id), _csv_escape(_session_timestamp_utc), Engine.physics_ticks_per_second,
        _csv_escape(_test_id()), _csv_escape(_current_scene_path()), _csv_escape(str(vehicle.get_path())), _csv_escape(_vehicle_scene_path()),
        _csv_escape(_vehicle_script_path()), 1, _csv_escape(_setup_json),
        int((int(vehicle.aids_enabled_mask) & 2) != 0)
    ]

func _csv_escape(value: String) -> String:
    return "\"%s\"" % value.replace("\"", "\"\"")

func _build_setup_snapshot(telemetry_filename: String) -> Dictionary:
    return {
        "schema_version": 1,
        "session": {
            "telemetry_file": telemetry_filename,
            "session_id": _session_id,
            "timestamp_utc": _session_timestamp_utc,
            "physics_hz": Engine.physics_ticks_per_second,
            "test_id": _test_id()
        },
        "provenance": {
            "track_scene": _current_scene_path(),
            "vehicle_node_path": str(vehicle.get_path()),
            "vehicle_scene": _vehicle_scene_path(),
            "vehicle_script": _vehicle_script_path(),
            "engine_config": _safe_resource_path("engine_config"),
            "torque_curve": _safe_resource_path("torque_curve"),
            "git_commit": OS.get_environment("FORMULA90S_GIT_COMMIT"),
            "git_branch": OS.get_environment("FORMULA90S_GIT_BRANCH")
        },
        "vehicle": {
            "runtime_class": vehicle.get_class(),
            "configuration": ""
        },
        "chassis": _build_chassis_snapshot(),
        "tires": _snapshot_properties(["front_tire_radius", "front_tire_width", "front_wheel_mass", "rear_tire_radius", "rear_tire_width", "rear_wheel_mass", "contact_patch", "braking_grip_multiplier", "wheel_to_body_torque_multiplier", "tire_stiffnesses", "coefficient_of_friction", "rolling_resistance", "lateral_grip_assist", "longitudinal_grip_ratio"]),
        "steering": _snapshot_properties(["steering_speed", "countersteer_speed", "steering_speed_decay", "steering_slip_assist", "countersteer_assist", "steering_exponent", "max_steering_angle", "front_steering_ratio", "rear_steering_ratio"]),
        "brakes": _snapshot_properties(["braking_speed", "brake_force_multiplier", "front_brake_bias", "traction_control_max_slip", "front_abs_pulse_time", "front_abs_spin_difference_threshold", "rear_abs_pulse_time", "rear_abs_spin_difference_threshold"]),
        "differential": _snapshot_properties(["front_torque_split", "variable_torque_split", "front_variable_split", "variable_split_speed", "front_locking_differential_engage_torque", "rear_locking_differential_engage_torque", "front_torque_vectoring", "rear_torque_vectoring"]),
        "suspension": _snapshot_properties(["front_spring_length", "front_resting_ratio", "front_damping_ratio", "front_bump_damp_multiplier", "front_rebound_damp_multiplier", "front_arb_ratio", "front_camber", "front_toe", "front_bump_stop_multiplier", "front_beam_axle", "rear_spring_length", "rear_resting_ratio", "rear_damping_ratio", "rear_bump_damp_multiplier", "rear_rebound_damp_multiplier", "rear_arb_ratio", "rear_camber", "rear_toe", "rear_bump_stop_multiplier", "rear_beam_axle"]),
        "engine": _snapshot_properties(["max_torque", "max_rpm", "idle_rpm", "motor_drag", "motor_brake", "motor_moment", "clutch_out_rpm", "max_clutch_torque_ratio", "throttle_speed", "throttle_steering_adjust"]),
        "transmission": _snapshot_properties(["gear_ratios", "final_drive", "reverse_ratio", "shift_time", "automatic_transmission", "automatic_time_between_shifts", "gear_inertia"]),
        "aerodynamics": _snapshot_properties(["coefficient_of_drag", "air_density", "frontal_area"]),
        "assists": _snapshot_properties(["enable_stability", "stability_yaw_engage_angle", "stability_yaw_strength", "stability_yaw_ground_multiplier", "stability_upright_spring", "stability_upright_damping", "automatic_transmission", "steering_slip_assist", "countersteer_assist"])
    }

func _snapshot_properties(property_names: Array[String]) -> Dictionary:
    var snapshot := {}
    for property_name in property_names:
        snapshot[property_name] = vehicle.get(property_name)
    return snapshot

func _build_chassis_snapshot() -> Dictionary:
    var snapshot := _snapshot_properties(["vehicle_mass", "front_weight_distribution", "center_of_gravity_height_offset", "inertia_multiplier"])
    snapshot["rigid_body_mass"] = vehicle.mass
    if vehicle.front_left_wheel and vehicle.front_right_wheel and vehicle.rear_left_wheel and vehicle.rear_right_wheel:
        var front_center: Vector3 = (vehicle.front_left_wheel.position + vehicle.front_right_wheel.position) * 0.5
        var rear_center: Vector3 = (vehicle.rear_left_wheel.position + vehicle.rear_right_wheel.position) * 0.5
        snapshot["wheelbase_m"] = absf(front_center.z - rear_center.z)
        snapshot["front_track_m"] = absf(vehicle.front_left_wheel.position.x - vehicle.front_right_wheel.position.x)
        snapshot["rear_track_m"] = absf(vehicle.rear_left_wheel.position.x - vehicle.rear_right_wheel.position.x)
    return snapshot

func _current_scene_path() -> String:
    var scene := get_tree().current_scene
    return String(scene.scene_file_path) if scene else ""

func _test_id() -> String:
    var scene_path := _current_scene_path()
    return scene_path.get_file().get_basename() if not scene_path.is_empty() else ""

func _vehicle_scene_path() -> String:
    var scene_path := String(vehicle.scene_file_path)
    if not scene_path.is_empty():
        return scene_path
    var parent: Node = vehicle.get_parent()
    return String(parent.scene_file_path) if parent else ""

func _vehicle_script_path() -> String:
    var script := vehicle.get_script() as Script
    return script.resource_path if script else ""

func _resource_path(resource: Resource) -> String:
    return resource.resource_path if resource else ""

# Reads a vehicle property that may not exist on both routes (GEVP `Vehicle` vs
# Rust `F194RustVehicle`) and returns its resource path, or "" when absent.
func _safe_resource_path(property_name: String) -> String:
    if vehicle == null:
        return ""
    var value = vehicle.get(property_name)
    if value == null:
        return ""
    var res := value as Resource
    return res.resource_path if res else ""

func _flush():
    if _file and _buffer.size() > 0:
        _file.store_string("\n".join(_buffer) + "\n")
        _buffer.clear()
