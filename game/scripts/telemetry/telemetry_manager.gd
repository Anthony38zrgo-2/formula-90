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
const F1_2026_2008_PHYSICS_CONFIG := "res://data/vehicles/f1_2026_2008/f1_2026_2008_physics.json"
const CSV_COLUMNS := [
    "Time_ms", "Speed_kmh", "RPM", "Gear",
    "Throttle", "Brake", "Steering",
    "Lat_G", "Long_G",
    "FL_Comp", "FR_Comp", "RL_Comp", "RR_Comp",
    "Front_Slip", "Rear_Slip",
    "Session_Id", "Session_Timestamp_UTC", "Physics_Hz",
    "Test_Id", "Track_Scene", "Vehicle_Node_Path", "Vehicle_Scene",
    "Vehicle_Script", "Setup_Schema_Version", "Setup_JSON",
    "TC_Enabled", "TC_Eligible", "TC_Active", "TC_GearAuthority", "TC_SlipTarget", "TC_RawCutRatio", "TC_CutRatio", "TC_AppliedCutRatio",
    "RL_TCSlip", "RR_TCSlip",
    "FL_DriveTorquePreTC_Nm", "FR_DriveTorquePreTC_Nm", "RL_DriveTorquePreTC_Nm", "RR_DriveTorquePreTC_Nm",
    "FL_DriveTorque_Nm", "FR_DriveTorque_Nm", "RL_DriveTorque_Nm", "RR_DriveTorque_Nm", "DriveTorque_Nm", "PreTCDrivePower_W", "NetDrivePower_W",
    "FL_BrakeTorque_Nm", "FL_SpinPre_RadS", "FL_SpinPost_RadS", "FL_BrakePower_W", "FL_BrakeEnergy_J",
    "FR_BrakeTorque_Nm", "FR_SpinPre_RadS", "FR_SpinPost_RadS", "FR_BrakePower_W", "FR_BrakeEnergy_J",
    "RL_BrakeTorque_Nm", "RL_SpinPre_RadS", "RL_SpinPost_RadS", "RL_BrakePower_W", "RL_BrakeEnergy_J",
    "RR_BrakeTorque_Nm", "RR_SpinPre_RadS", "RR_SpinPost_RadS", "RR_BrakePower_W", "RR_BrakeEnergy_J",
    "FL_TreadInner_C", "FL_TreadCenter_C", "FL_TreadOuter_C", "FL_Carcass_C", "FL_Gas_C", "FL_Disc_C", "FL_Rim_C", "FL_BrakeEfficiency", "FL_DuctMassFlow_kg_s", "FL_DuctDrag_N",
    "FR_TreadInner_C", "FR_TreadCenter_C", "FR_TreadOuter_C", "FR_Carcass_C", "FR_Gas_C", "FR_Disc_C", "FR_Rim_C", "FR_BrakeEfficiency", "FR_DuctMassFlow_kg_s", "FR_DuctDrag_N",
    "RL_TreadInner_C", "RL_TreadCenter_C", "RL_TreadOuter_C", "RL_Carcass_C", "RL_Gas_C", "RL_Disc_C", "RL_Rim_C", "RL_BrakeEfficiency", "RL_DuctMassFlow_kg_s", "RL_DuctDrag_N",
    "RR_TreadInner_C", "RR_TreadCenter_C", "RR_TreadOuter_C", "RR_Carcass_C", "RR_Gas_C", "RR_Disc_C", "RR_Rim_C", "RR_BrakeEfficiency", "RR_DuctMassFlow_kg_s", "RR_DuctDrag_N",
    "FL_NaturalCooling_WK", "FR_NaturalCooling_WK", "RL_NaturalCooling_WK", "RR_NaturalCooling_WK",
    "FL_SpeedCooling_WK", "FR_SpeedCooling_WK", "RL_SpeedCooling_WK", "RR_SpeedCooling_WK",
    "UF_FL_Clearance_m", "UF_FR_Clearance_m", "UF_Center_Clearance_m", "UF_DiffuserThroat_Clearance_m", "UF_DiffuserExit_Clearance_m",
    "UF_ValidMask", "UF_ScrapePhase", "UF_MinClearance_m", "UF_Rake_rad", "UF_Roll_rad",
    "UF_ContactConfidence", "UF_ScrapeIntensity", "UF_AudioGain", "UF_AudioPitch", "UF_AudioCursor",
    "UF_FL_Compression_m", "UF_FR_Compression_m", "UF_Center_Compression_m", "UF_DiffuserThroat_Compression_m", "UF_DiffuserExit_Compression_m",
    "UF_FL_ClosingSpeed_mps", "UF_FR_ClosingSpeed_mps", "UF_Center_ClosingSpeed_mps", "UF_DiffuserThroat_ClosingSpeed_mps", "UF_DiffuserExit_ClosingSpeed_mps",
    "UF_FL_Force_N", "UF_FR_Force_N", "UF_Center_Force_N", "UF_DiffuserThroat_Force_N", "UF_DiffuserExit_Force_N",
    "UF_FL_BottomingPhase", "UF_FR_BottomingPhase", "UF_Center_BottomingPhase", "UF_DiffuserThroat_BottomingPhase", "UF_DiffuserExit_BottomingPhase",
    "UF_ActiveProbeMask", "UF_TotalNormalForce_N", "UF_MaxProbeForce_N", "UF_DissipatedEnergy_J", "UF_RigidContactBlend",
    "Aero_TotalDownforce_N", "Aero_RawDownforce_N", "Aero_FrontDownforce_N", "Aero_FloorDownforce_N", "Aero_RearDownforce_N", "Aero_Drag_N",
    "Aero_FrontWingAngle_deg", "Aero_RearWingAngle_deg", "Aero_FrontWing_CL", "Aero_RearWing_CL",
    "Aero_FloorHeightFactor", "Aero_FloorRakeFactor", "Aero_FloorSealFactor", "Aero_DiffuserExpansion_deg", "Aero_DiffuserStallFactor",
    "Aero_GlobalLimitFactor", "Aero_LoadRatio", "Aero_BalanceFront"
]

func _ready():
    _prev_sample_time = Time.get_ticks_msec()
    _prev_velocity_time = _prev_sample_time

func _physics_process(delta):
    if not enabled:
        return

    if not vehicle:
        _try_find_vehicle()
        if not vehicle:
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
    var tc_enabled := false
    var tc_eligible := false
    var tc_intervening := false
    var tc_gear_authority := 0.0
    var tc_slip_target := 0.0
    var tc_raw_cut_ratio := 0.0
    var tc_cut_ratio := 0.0
    var tc_slip: Array = [0.0, 0.0, 0.0, 0.0]
    var drive_torque_pre_tc: Array = [0.0, 0.0, 0.0, 0.0]
    var drive_torque: Array = [0.0, 0.0, 0.0, 0.0]
    var pre_tc_drive_power_w := 0.0
    var net_drive_power_w := 0.0

    if _is_rust:
        var comp = vehicle.get_wheel_compressions()
        if comp.size() >= 4:
            fl_comp = comp[0]; fr_comp = comp[1]; rl_comp = comp[2]; rr_comp = comp[3]
        var slips = vehicle.get_wheel_slips()
        if slips.size() >= 4:
            front_slip = maxf(absf(slips[0]), absf(slips[1]))
            rear_slip = maxf(absf(slips[2]), absf(slips[3]))
        if vehicle.has_method(&"get_powertrain_state_snapshot"):
            var powertrain_value: Variant = vehicle.call(&"get_powertrain_state_snapshot")
            if powertrain_value is Dictionary:
                var powertrain: Dictionary = powertrain_value
                tc_enabled = bool(powertrain.get("tc_enabled", false))
                tc_eligible = bool(powertrain.get("tc_eligible", false))
                tc_intervening = bool(powertrain.get("tc_intervening", false))
                tc_gear_authority = float(powertrain.get("tc_gear_authority", 0.0))
                tc_slip_target = float(powertrain.get("tc_slip_target", 0.0))
                tc_raw_cut_ratio = float(powertrain.get("tc_raw_cut_ratio", 0.0))
                tc_cut_ratio = float(powertrain.get("tc_cut_ratio", 0.0))
                pre_tc_drive_power_w = float(powertrain.get("pre_tc_drive_power_w", 0.0))
                net_drive_power_w = float(powertrain.get("net_drive_power_w", 0.0))
                var tc_slip_value: Variant = powertrain.get("tc_slip_ratio", PackedFloat64Array())
                var pre_tc_torque_value: Variant = powertrain.get("wheel_drive_torque_pre_tc_nm", PackedFloat64Array())
                var torque_value: Variant = powertrain.get("wheel_drive_torque_nm", PackedFloat64Array())
                if tc_slip_value is PackedFloat64Array and tc_slip_value.size() >= 4:
                    for i in range(4):
                        tc_slip[i] = float(tc_slip_value[i])
                if pre_tc_torque_value is PackedFloat64Array and pre_tc_torque_value.size() >= 4:
                    for i in range(4):
                        drive_torque_pre_tc[i] = float(pre_tc_torque_value[i])
                if torque_value is PackedFloat64Array and torque_value.size() >= 4:
                    for i in range(4):
                        drive_torque[i] = float(torque_value[i])
    else:
        fl_comp = vehicle.front_axle.suspension_compression_left if vehicle.front_axle else 0.0
        fr_comp = vehicle.front_axle.suspension_compression_right if vehicle.front_axle else 0.0
        rl_comp = vehicle.rear_axle.suspension_compression_left if vehicle.rear_axle else 0.0
        rr_comp = vehicle.rear_axle.suspension_compression_right if vehicle.rear_axle else 0.0
        front_slip = vehicle.front_axle.get_max_wheel_slip_y() if vehicle.front_axle else 0.0
        rear_slip = vehicle.rear_axle.get_max_wheel_slip_y() if vehicle.rear_axle else 0.0

    var brake_torque: Array = [0.0, 0.0, 0.0, 0.0]
    var spin_pre: Array = [0.0, 0.0, 0.0, 0.0]
    var spin_post: Array = [0.0, 0.0, 0.0, 0.0]
    var brake_power: Array = [0.0, 0.0, 0.0, 0.0]
    var brake_energy: Array = [0.0, 0.0, 0.0, 0.0]
    var resolved: Array = []
    for _field in range(8):
        resolved.append(0.0)
    # Per wheel: tread I/C/O, carcass, gas, disc, rim,
    # brake efficiency, duct mass flow, duct drag.
    var thermal: Array = []
    for _field in range(40):
        thermal.append(0.0)
    var underfloor_fields: Array = [0.35, 0.35, 0.35, 0.35, 0.35, 0, 0, 0.35, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0, 0, 0, 0, 0, 0, 0.0, 0.0, 0.0, 0.0]
    var aero_fields: Array = []
    aero_fields.resize(17)
    aero_fields.fill(0.0)
    if _is_rust:
        var snapshot_value: Variant = vehicle.get_telemetry_snapshot()
        if snapshot_value is Dictionary:
            var tire_state: Dictionary = snapshot_value.get("tires", {})
            var brakes_value: Variant = snapshot_value.get("brakes", {})
            for i in range(4):
                var wheel_name: String = ["FL", "FR", "RL", "RR"][i]
                var offset := i * 10
                if tire_state.has(wheel_name):
                    var tire_value: Variant = tire_state.get(wheel_name, {})
                    if tire_value is Dictionary:
                        var tire: Dictionary = tire_value
                        thermal[offset] = float(tire.get("tread_inner_c", 0.0))
                        thermal[offset + 1] = float(tire.get("tread_center_c", 0.0))
                        thermal[offset + 2] = float(tire.get("tread_outer_c", 0.0))
                        thermal[offset + 3] = float(tire.get("carcass_c", 0.0))
                        thermal[offset + 4] = float(tire.get("gas_c", 0.0))
                if brakes_value is Dictionary:
                    var brake_state: Dictionary = brakes_value
                    var wheel_value: Variant = brake_state.get(wheel_name, {})
                    if wheel_value is Dictionary:
                        var wheel_state: Dictionary = wheel_value
                        brake_torque[i] = float(wheel_state.get("brake_torque_nm", 0.0))
                        spin_pre[i] = float(wheel_state.get("spin_pre_rad_s", 0.0))
                        spin_post[i] = float(wheel_state.get("spin_post_rad_s", 0.0))
                        brake_power[i] = float(wheel_state.get("brake_power_w", 0.0))
                        brake_energy[i] = float(wheel_state.get("brake_energy_j", 0.0))
                        resolved[i] = float(wheel_state.get("natural_cooling_w_k", 0.0))
                        resolved[4 + i] = float(wheel_state.get("speed_cooling_w_k", 0.0))
                        thermal[offset + 5] = float(wheel_state.get("disc_c", 0.0))
                        thermal[offset + 6] = float(wheel_state.get("rim_c", 0.0))
                        thermal[offset + 7] = float(wheel_state.get("efficiency", 0.0))
                        thermal[offset + 8] = float(wheel_state.get("duct_mass_flow_kg_s", 0.0))
                        thermal[offset + 9] = float(wheel_state.get("duct_drag_n", 0.0))
        if vehicle.has_method(&"get_underfloor_state_snapshot"):
            var underfloor_value: Variant = vehicle.call(&"get_underfloor_state_snapshot")
            if underfloor_value is Dictionary:
                var underfloor: Dictionary = underfloor_value
                var clearances_value: Variant = underfloor.get("clearance_m", {})
                if clearances_value is Dictionary:
                    var clearances: Dictionary = clearances_value
                    underfloor_fields[0] = float(clearances.get("front_left", 0.35))
                    underfloor_fields[1] = float(clearances.get("front_right", 0.35))
                    underfloor_fields[2] = float(clearances.get("center", 0.35))
                    underfloor_fields[3] = float(clearances.get("diffuser_throat", 0.35))
                    underfloor_fields[4] = float(clearances.get("diffuser_exit", 0.35))
                underfloor_fields[5] = int(underfloor.get("valid_mask", 0))
                underfloor_fields[6] = int(underfloor.get("scrape_phase", 0))
                underfloor_fields[7] = float(underfloor.get("minimum_clearance_m", 0.35))
                underfloor_fields[8] = float(underfloor.get("rake_rad", 0.0))
                underfloor_fields[9] = float(underfloor.get("roll_rad", 0.0))
                underfloor_fields[10] = float(underfloor.get("contact_confidence", 0.0))
                underfloor_fields[11] = float(underfloor.get("scrape_intensity", 0.0))
                underfloor_fields[12] = float(underfloor.get("audio_scrape_gain", 0.0))
                underfloor_fields[13] = float(underfloor.get("audio_scrape_pitch", 1.0))
                underfloor_fields[14] = int(underfloor.get("audio_scrape_cursor", 0))
                var probe_names := ["front_left", "front_right", "center", "diffuser_throat", "diffuser_exit"]
                var compression: Dictionary = underfloor.get("compression_m", {})
                var closing_speed: Dictionary = underfloor.get("closing_speed_m_s", {})
                var normal_force: Dictionary = underfloor.get("normal_force_n", {})
                var bottoming_phase: Dictionary = underfloor.get("bottoming_phase", {})
                for i in range(5):
                    underfloor_fields[15 + i] = float(compression.get(probe_names[i], 0.0))
                    underfloor_fields[20 + i] = float(closing_speed.get(probe_names[i], 0.0))
                    underfloor_fields[25 + i] = float(normal_force.get(probe_names[i], 0.0))
                    underfloor_fields[30 + i] = int(bottoming_phase.get(probe_names[i], 0))
                underfloor_fields[35] = int(underfloor.get("active_probe_mask", 0))
                underfloor_fields[36] = float(underfloor.get("total_normal_force_n", 0.0))
                underfloor_fields[37] = float(underfloor.get("max_probe_force_n", 0.0))
                underfloor_fields[38] = float(underfloor.get("dissipated_energy_j", 0.0))
                underfloor_fields[39] = float(underfloor.get("rigid_contact_blend", 0.0))
                var aero_value: Variant = underfloor.get("aero", {})
                if aero_value is Dictionary:
                    var aero: Dictionary = aero_value
                    var aero_names := ["total_downforce_n", "raw_downforce_n", "front_downforce_n", "floor_downforce_n", "rear_downforce_n", "drag_n", "front_wing_angle_deg", "rear_wing_angle_deg", "front_wing_cl", "rear_wing_cl", "floor_height_factor", "floor_rake_factor", "floor_seal_factor", "diffuser_stall_factor", "global_limit_factor", "load_ratio", "balance_front"]
                    for i in range(aero_names.size()):
                        aero_fields[i] = float(aero.get(aero_names[i], 0.0))

    var base_fields := PackedStringArray([
        "%d" % now_msec, "%.1f" % speed_kmh, "%d" % rpm, "%d" % gear,
        "%.3f" % throttle, "%.3f" % brake_amt, "%.3f" % steering,
        "%.3f" % lat_g, "%.3f" % long_g,
        "%.1f" % fl_comp, "%.1f" % fr_comp, "%.1f" % rl_comp, "%.1f" % rr_comp,
        "%.3f" % front_slip, "%.3f" % rear_slip,
        _csv_escape(_session_id), _csv_escape(_session_timestamp_utc), "%d" % Engine.physics_ticks_per_second,
        _csv_escape(_test_id()), _csv_escape(_current_scene_path()), _csv_escape(str(vehicle.get_path())), _csv_escape(_vehicle_scene_path()),
        _csv_escape(_vehicle_script_path()), "1", _csv_escape(_setup_json),
        "%d" % int(tc_enabled), "%d" % int(tc_eligible), "%d" % int(tc_intervening),
        "%.4f" % tc_gear_authority, "%.4f" % tc_slip_target, "%.4f" % tc_raw_cut_ratio,
        "%.4f" % tc_cut_ratio, "%.4f" % (tc_cut_ratio if tc_intervening else 0.0), "%.4f" % tc_slip[2], "%.4f" % tc_slip[3]
    ])
    for value in drive_torque_pre_tc:
        base_fields.append("%.3f" % float(value))
    for value in drive_torque:
        base_fields.append("%.3f" % float(value))
    base_fields.append("%.3f" % (float(drive_torque[0]) + float(drive_torque[1]) + float(drive_torque[2]) + float(drive_torque[3])))
    base_fields.append("%.3f" % pre_tc_drive_power_w)
    base_fields.append("%.3f" % net_drive_power_w)
    for i in range(4):
        base_fields.append("%.3f" % float(brake_torque[i]))
        base_fields.append("%.3f" % float(spin_pre[i]))
        base_fields.append("%.3f" % float(spin_post[i]))
        base_fields.append("%.3f" % float(brake_power[i]))
        base_fields.append("%.3f" % float(brake_energy[i]))
    return ",".join(base_fields) + "," + ",".join(_thermal_csv_fields(thermal)) + "," + ",".join(_thermal_csv_fields(resolved)) + "," + ",".join(_underfloor_csv_fields(underfloor_fields)) + "," + ",".join(_thermal_csv_fields(aero_fields))

func _thermal_csv_fields(values: Array) -> PackedStringArray:
    var fields := PackedStringArray()
    for value in values:
        fields.append("%.4f" % float(value))
    return fields

func _underfloor_csv_fields(values: Array) -> PackedStringArray:
    var fields := PackedStringArray()
    for i in range(values.size()):
        if i == 5 or i == 6 or i == 14 or (i >= 30 and i <= 35):
            fields.append("%d" % int(values[i]))
        else:
            fields.append("%.6f" % float(values[i]))
    return fields

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
            "engine_config": _get_active_physics_config_path() if _is_rust else _safe_resource_path("engine_config"),
            "torque_curve": _safe_resource_path("torque_curve"),
            "git_commit": OS.get_environment("FORMULA90S_GIT_COMMIT"),
            "git_branch": OS.get_environment("FORMULA90S_GIT_BRANCH")
        },
        "vehicle": {
            "runtime_class": vehicle.get_class(),
            "configuration": _get_active_physics_config_path()
        },
        "chassis": _build_chassis_snapshot(),
        "underfloor": _build_underfloor_snapshot(),
        "tires": _snapshot_properties(["front_tire_radius", "front_tire_width", "front_wheel_mass", "rear_tire_radius", "rear_tire_width", "rear_wheel_mass", "contact_patch", "braking_grip_multiplier", "wheel_to_body_torque_multiplier", "tire_stiffnesses", "coefficient_of_friction", "rolling_resistance", "lateral_grip_assist", "longitudinal_grip_ratio"]),
        "thermal": _build_brake_thermal_snapshot(),
        "steering": _snapshot_properties(["steering_speed", "countersteer_speed", "steering_speed_decay", "steering_slip_assist", "countersteer_assist", "steering_exponent", "max_steering_angle", "front_steering_ratio", "rear_steering_ratio"]),
        "brakes": _snapshot_properties(["braking_speed", "brake_force_multiplier", "front_brake_bias", "traction_control_max_slip", "front_abs_pulse_time", "front_abs_spin_difference_threshold", "rear_abs_pulse_time", "rear_abs_spin_difference_threshold"]),
        "differential": _snapshot_properties(["front_torque_split", "variable_torque_split", "front_variable_split", "variable_split_speed", "front_locking_differential_engage_torque", "rear_locking_differential_engage_torque", "front_torque_vectoring", "rear_torque_vectoring"]),
        "suspension": _snapshot_properties(["front_spring_length", "front_resting_ratio", "front_damping_ratio", "front_bump_damp_multiplier", "front_rebound_damp_multiplier", "front_arb_ratio", "front_camber", "front_toe", "front_bump_stop_multiplier", "front_beam_axle", "rear_spring_length", "rear_resting_ratio", "rear_damping_ratio", "rear_bump_damp_multiplier", "rear_rebound_damp_multiplier", "rear_arb_ratio", "rear_camber", "rear_toe", "rear_bump_stop_multiplier", "rear_beam_axle"]),
        "engine": _build_engine_snapshot(),
        "transmission": _build_transmission_snapshot(),
        "aerodynamics": _snapshot_properties(["coefficient_of_drag", "air_density", "frontal_area"]),
        "assists": _snapshot_properties(["enable_stability", "stability_yaw_engage_angle", "stability_yaw_strength", "stability_yaw_ground_multiplier", "stability_upright_spring", "stability_upright_damping", "automatic_transmission", "steering_slip_assist", "countersteer_assist"])
    }

func _get_active_physics_config_path() -> String:
    if vehicle != null:
        var cfg_path = vehicle.get("physics_config_path")
        if cfg_path is String and not cfg_path.is_empty():
            return cfg_path
        if vehicle.has_method("get_physics_config_path"):
            var p: String = vehicle.call("get_physics_config_path")
            if not p.is_empty():
                return p
    var f90_core = get_tree().root.find_children("*", "F90Core", true, false)
    if f90_core.size() > 0:
        var c_path = f90_core[0].get("config_json_path")
        if c_path is String and not c_path.is_empty():
            return c_path
    return F1_2026_2008_PHYSICS_CONFIG

func _build_engine_snapshot() -> Dictionary:
    var fields: Array[String] = ["max_torque", "max_rpm", "idle_rpm", "motor_drag", "motor_brake", "motor_moment", "clutch_out_rpm", "max_clutch_torque_ratio", "throttle_speed", "throttle_steering_adjust"]
    var snapshot := _snapshot_properties(fields)
    var config_path := _get_active_physics_config_path()
    if _is_rust and FileAccess.file_exists(config_path):
        var file := FileAccess.open(config_path, FileAccess.READ)
        if file != null:
            var parsed: Variant = JSON.parse_string(file.get_as_text())
            if parsed is Dictionary:
                var powertrain: Variant = parsed.get("powertrain", {})
                if powertrain is Dictionary:
                    for field in fields:
                        if snapshot.get(field) == null and powertrain.has(field):
                            snapshot[field] = powertrain[field]
    return snapshot

func _build_brake_thermal_snapshot() -> Dictionary:
    var snapshot := {
        "model": "runtime_resolved",
        "model_source": "vehicle_physics_engine brake thermal resolver",
        "resolved_wheels": {}
    }
    if vehicle.has_method(&"get_brake_state_snapshot"):
        var value: Variant = vehicle.call(&"get_brake_state_snapshot")
        if value is Dictionary:
            snapshot["resolved_wheels"] = value
    return snapshot

func _build_underfloor_snapshot() -> Dictionary:
    if vehicle.has_method(&"get_underfloor_state_snapshot"):
        var value: Variant = vehicle.call(&"get_underfloor_state_snapshot")
        if value is Dictionary:
            return value
    return {}

func _snapshot_properties(property_names: Array[String]) -> Dictionary:
    var snapshot := {}
    for property_name in property_names:
        snapshot[property_name] = vehicle.get(property_name)
    return snapshot

func _build_transmission_snapshot() -> Dictionary:
    var fields: Array[String] = ["gear_ratios", "final_drive", "reverse_ratio", "shift_time", "automatic_transmission", "automatic_time_between_shifts", "gear_inertia"]
    var snapshot := _snapshot_properties(fields)
    var config_path := _get_active_physics_config_path()
    if not _is_rust or not FileAccess.file_exists(config_path):
        return snapshot

    var file := FileAccess.open(config_path, FileAccess.READ)
    if file == null:
        return snapshot
    var parsed: Variant = JSON.parse_string(file.get_as_text())
    if not (parsed is Dictionary):
        return snapshot
    var powertrain: Variant = parsed.get("powertrain", {})
    if not (powertrain is Dictionary):
        return snapshot
    for field in fields:
        if snapshot[field] == null and powertrain.has(field):
            snapshot[field] = powertrain[field]
    return snapshot

func _build_chassis_snapshot() -> Dictionary:
    var snapshot := _snapshot_properties(["vehicle_mass", "front_weight_distribution", "center_of_gravity_height_offset", "inertia_multiplier"])
    snapshot["rigid_body_mass"] = vehicle.mass
    var config_path := _get_active_physics_config_path()
    if _is_rust and FileAccess.file_exists(config_path):
        var file := FileAccess.open(config_path, FileAccess.READ)
        if file != null:
            var parsed: Variant = JSON.parse_string(file.get_as_text())
            if parsed is Dictionary:
                var chassis: Variant = parsed.get("chassis", {})
                if chassis is Dictionary:
                    if snapshot.get("vehicle_mass") == null and chassis.has("vehicle_mass"):
                        snapshot["vehicle_mass"] = chassis["vehicle_mass"]
                    if snapshot.get("front_weight_distribution") == null and chassis.has("front_weight_distribution"):
                        snapshot["front_weight_distribution"] = chassis["front_weight_distribution"]
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
