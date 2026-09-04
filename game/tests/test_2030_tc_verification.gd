extends SceneTree

const SCENE_PATH := "res://scenes/runtime/vehicle_test_session_2026.tscn"

func _init() -> void:
	call_deferred("_run")

func _run() -> void:
	print("=== INICIANDO TEST VERIFICACION TC & PERFIL 2030 ===")
	var packed := load(SCENE_PATH) as PackedScene
	if packed == null:
		printerr("[FAIL] No se pudo cargar ", SCENE_PATH)
		quit(1)
		return

	var root_node = packed.instantiate()
	root.add_child(root_node)

	for _i in range(10):
		await process_frame

	var vehicle = root_node.find_child("VehicleRigidBody", true, false)
	if vehicle == null:
		printerr("[FAIL] VehicleRigidBody no encontrado")
		quit(1)
		return

	var aids_ctrl: DrivingAidsController = null
	var aids_nodes = root_node.find_children("*", "DrivingAidsController", true, false)
	if aids_nodes.size() > 0:
		aids_ctrl = aids_nodes[0] as DrivingAidsController

	if aids_ctrl == null:
		printerr("[FAIL] DrivingAidsController no encontrado")
		quit(1)
		return

	# --- VERIFICACIÓN 5: PERFIL 2030 Y SETUP_JSON ---
	print("--- 5. Verificando perfil 2030 y Setup_JSON ---")
	for _i in range(10):
		await physics_frame
	var telem_mgr = root.get_node_or_null("/root/TelemetryManager")
	if telem_mgr == null:
		printerr("[FAIL] TelemetryManager no encontrado en root")
		quit(1)
		return

	var setup_str: String = telem_mgr.get("_setup_json")
	var setup: Dictionary = JSON.parse_string(setup_str) if not setup_str.is_empty() else {}
	if setup.is_empty():
		printerr("[FAIL] Setup_JSON vacio")
		quit(1)
		return

	var prov: Dictionary = setup.get("provenance", {})
	var veh_info: Dictionary = setup.get("vehicle", {})
	var engine: Dictionary = setup.get("engine", {})
	var trans: Dictionary = setup.get("transmission", {})
	var chassis: Dictionary = setup.get("chassis", {})

	print("Provenance engine_config: ", prov.get("engine_config"))
	print("Vehicle configuration: ", veh_info.get("configuration"))
	print("RigidBody mass: ", vehicle.mass, " chassis.vehicle_mass: ", chassis.get("vehicle_mass"))
	print("Engine max_torque: ", engine.get("max_torque"), " max_rpm: ", engine.get("max_rpm"))
	print("Transmission final_drive: ", trans.get("final_drive"), " gear_ratios: ", trans.get("gear_ratios"))

	var engine_cfg_path: String = str(prov.get("engine_config", ""))
	var veh_cfg_path: String = str(veh_info.get("configuration", ""))
	assert(not engine_cfg_path.is_empty(), "engine_config no debe estar vacio")
	assert(not veh_cfg_path.is_empty(), "vehicle.configuration no debe estar vacio")
	assert(engine_cfg_path.contains("f1_2026_2008_physics.json"), "engine_config debe apuntar al JSON 2030/2026")
	assert(float(chassis.get("vehicle_mass", 0.0)) == 650.0 or vehicle.mass == 650.0, "Masa debe ser 650 kg")
	assert(float(engine.get("max_torque", 0.0)) == 410.0, "Torque maximo debe ser 410 Nm")
	assert(float(engine.get("max_rpm", 0.0)) == 17500.0, "RPM maxima debe ser 17500 rpm")
	assert(float(trans.get("final_drive", 0.0)) == 4.25, "Final Drive debe ser 4.25")
	var ratios: Array = trans.get("gear_ratios", [])
	assert(ratios.size() == 7, "Debe tener 7 marchas")
	print("[PASS] Verificación 5: Perfil 2030 y Setup_JSON validados con éxito!")

	# --- VERIFICACIÓN 1: ENLACE UI -> RUNTIME DEL TC ---
	print("--- 1. Verificando enlace UI -> Runtime TC ---")
	var pt_initial: Dictionary = vehicle.call(&"get_powertrain_state_snapshot")
	print("TC_Enabled inicial: ", pt_initial.get("tc_enabled"))
	assert(bool(pt_initial.get("tc_enabled", false)) == false, "TC debe iniciar deshabilitado por defecto")

	# Simular selección de TC en juego / UI vía aids_ctrl.toggle(4)
	print("Activando TC vía DrivingAidsController...")
	aids_ctrl.toggle(4)
	assert(aids_ctrl.is_aid_enabled(4) == true, "DrivingAidsController aid[4] debe estar activo")

	# Dar un tick para que el frame de simulación ejecute
	await physics_frame
	await physics_frame

	var pt_toggled: Dictionary = vehicle.call(&"get_powertrain_state_snapshot")
	print("TC_Enabled tras toggle: ", pt_toggled.get("tc_enabled"))
	assert(bool(pt_toggled.get("tc_enabled", false)) == true, "TC_Enabled en runtime debe cambiar inmediatamente de 0 a 1")
	print("[PASS] Verificación 1: Enlace UI -> Runtime TC validado con éxito!")

	# --- VERIFICACIÓN 2: TC_Eligible ---
	print("--- 2. Verificando TC_Eligible ---")
	# Desactivar input del jugador para controlar acelerador directamente
	vehicle.enable_player_input = false
	vehicle.throttle_amount = 1.0

	await physics_frame
	await physics_frame

	var pt_eligible: Dictionary = vehicle.call(&"get_powertrain_state_snapshot")
	print("Gear: ", vehicle.current_gear, " Throttle: ", vehicle.throttle_amount, " TC_Eligible: ", pt_eligible.get("tc_eligible"))
	assert(bool(pt_eligible.get("tc_eligible", false)) == true, "TC_Eligible debe ser 1 con TC habilitado, RWD, marcha engranada y acelerador aplicado")
	print("[PASS] Verificación 2: TC_Eligible = 1 validado con éxito!")

	# --- VERIFICACIÓN 3 & 4: TC_Active, Cut Ratios y Reducción de Torque ---
	print("--- 3 & 4. Verificando intervención TC y reducción de torque ---")
	var tc_intervened := false
	var verified_cuts := false
	var verified_torque_reduction := false

	for step in range(300):
		vehicle.throttle_amount = 1.0
		await physics_frame
		var pt: Dictionary = vehicle.call(&"get_powertrain_state_snapshot")
		var tc_active = bool(pt.get("tc_intervening", false))
		var raw_cut = float(pt.get("tc_raw_cut_ratio", 0.0))
		var cut_ratio = float(pt.get("tc_cut_ratio", 0.0))
		var pre_torques: PackedFloat64Array = pt.get("wheel_drive_torque_pre_tc_nm", PackedFloat64Array())
		var post_torques: PackedFloat64Array = pt.get("wheel_drive_torque_nm", PackedFloat64Array())

		if tc_active:
			tc_intervened = true
			if raw_cut > 0.0 and cut_ratio > 0.0:
				verified_cuts = true
			var pre_total = (pre_torques[2] + pre_torques[3]) if pre_torques.size() >= 4 else 0.0
			var post_total = (post_torques[2] + post_torques[3]) if post_torques.size() >= 4 else 0.0
			if pre_total > post_total and post_total > 0.0:
				verified_torque_reduction = true
				print("Intervención detectada en step %d: TC_Active=1 RawCut=%.4f Cut=%.4f PreTorque=%.1fNm PostTorque=%.1fNm" % [
					step, raw_cut, cut_ratio, pre_total, post_total
				])
				# Run 60 more steps during intervention to log CSV rows
				for _extra in range(60):
					await physics_frame
				break

	assert(tc_intervened, "TC debe intervenir (TC_Active=1) bajo aceleración con slip")
	assert(verified_cuts, "TC_RawCutRatio > 0 y TC_CutRatio > 0 durante la intervención")
	assert(verified_torque_reduction, "DriveTorque_Nm < DriveTorquePreTC_Nm durante la intervención")
	print("[PASS] Verificación 3 & 4: Intervención de TC y reducción de torque validadas con éxito!")

	telem_mgr.call(&"_flush")
	print("=== TODAS LAS VERIFICACIONES COMPLETADAS EXITOSAMENTE ===")
	quit(0)
