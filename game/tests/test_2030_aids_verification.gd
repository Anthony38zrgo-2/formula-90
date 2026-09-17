extends SceneTree

## Verificacion end-to-end de ayudas del perfil F1 2030 V10:
##  1. ESP (estabilidad, bit2) inicia ACTIVO; TC (bit1) inicia deshabilitado.
##  2. Los toggles del DrivingAidsController mantienen la mascara coherente.
##  3. La tecla "Toggle Traction Control" produce EXACTAMENTE un flip por pulsacion
##     (regresion del doble handler input-controller/driving-aids).
##  4. El setup de telemetria reporta el estado real derivado de aids_enabled_mask.

const SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"
const TOGGLE_TC_ACTION := "Toggle Traction Control"
const MASK_BIT_TC := 1
const MASK_BIT_STABILITY := 2

func _init() -> void:
	call_deferred("_run")

func _fail(message: String) -> void:
	printerr("[FAIL] ", message)
	quit(1)

func _run() -> void:
	print("=== INICIANDO TEST VERIFICACION DE AYUDAS (ESP + TC) ===")
	var packed := load(SCENE_PATH) as PackedScene
	if packed == null:
		_fail("No se pudo cargar %s" % SCENE_PATH)
		return
	var root_node = packed.instantiate()
	root.add_child(root_node)
	for _i in range(10):
		await process_frame

	var vehicle = root_node.find_child("VehicleRigidBody", true, false)
	if vehicle == null:
		_fail("VehicleRigidBody no encontrado")
		return
	var aids_nodes = root_node.find_children("*", "DrivingAidsController", true, false)
	if aids_nodes.size() == 0:
		_fail("DrivingAidsController no encontrado")
		return
	var aids_ctrl: DrivingAidsController = aids_nodes[0] as DrivingAidsController

	for _i in range(10):
		await physics_frame

	# --- 1. Defaults ---
	var mask: int = int(vehicle.get("aids_enabled_mask"))
	var stability_bit := (mask & (1 << MASK_BIT_STABILITY)) != 0
	var tc_bit := (mask & (1 << MASK_BIT_TC)) != 0
	print("Mask inicial: %d (ESP=%s TC=%s)" % [mask, stability_bit, tc_bit])
	assert(stability_bit, "ESP debe iniciar activo (bit2=1)")
	assert(not tc_bit, "TC debe iniciar deshabilitado (bit1=0)")
	assert(aids_ctrl.is_aid_enabled(1) == stability_bit, "DrivingAidsController[1]=ESTAB debe coincidir con el bit2")
	assert(aids_ctrl.is_aid_enabled(4) == tc_bit, "DrivingAidsController[4]=TC debe coincidir con el bit1")
	print("[PASS] Defaults: ESP activo, TC deshabilitado.")

	# --- 2. Toggle ESP mantiene mascara y estado coherentes ---
	for cycle in range(3):
		aids_ctrl.toggle(1)
		await physics_frame
		mask = int(vehicle.get("aids_enabled_mask"))
		var expected_on: bool = aids_ctrl.is_aid_enabled(1)
		assert(((mask & (1 << MASK_BIT_STABILITY)) != 0) == expected_on,
			"mask ESTAB debe coincidir con el controller en el ciclo %d" % cycle)
	if not aids_ctrl.is_aid_enabled(1):
		aids_ctrl.toggle(1)
		await physics_frame
	mask = int(vehicle.get("aids_enabled_mask"))
	assert((mask & (1 << MASK_BIT_STABILITY)) != 0, "ESP debe quedar restaurado en ON")
	print("[PASS] Toggle ESP coherente con aids_enabled_mask.")

	# --- 3. Un solo flip por pulsacion de la tecla T ---
	vehicle.enable_player_input = true
	var before := int(vehicle.get("aids_enabled_mask"))
	Input.action_press(TOGGLE_TC_ACTION)
	await physics_frame
	Input.action_release(TOGGLE_TC_ACTION)
	await physics_frame
	var after := int(vehicle.get("aids_enabled_mask"))
	assert((before ^ after) == (1 << MASK_BIT_TC),
		"T debe producir exactamente un flip del bit TC (antes=%d despues=%d)" % [before, after])
	assert(aids_ctrl.is_aid_enabled(4) == ((after & (1 << MASK_BIT_TC)) != 0),
		"estado del controller TC debe coincidir con la mascara tras T")

	before = after
	Input.action_press(TOGGLE_TC_ACTION)
	await physics_frame
	Input.action_release(TOGGLE_TC_ACTION)
	await physics_frame
	after = int(vehicle.get("aids_enabled_mask"))
	assert((before ^ after) == (1 << MASK_BIT_TC),
		"cada pulsacion de T debe producir exactamente un flip (antes=%d despues=%d)" % [before, after])
	assert((after & (1 << MASK_BIT_TC)) == 0, "la segunda pulsacion devuelve TC a OFF")
	assert(aids_ctrl.is_aid_enabled(4) == false, "el controller TC debe volver a OFF")
	print("[PASS] Tecla T: un unico flip por pulsacion (sin doble handler).")

	# --- 4. Telemetria: assists derivados de la mascara (estado inicial) ---
	var telem_mgr = root.get_node_or_null("/root/TelemetryManager")
	if telem_mgr == null:
		_fail("TelemetryManager no encontrado en root")
		return
	var setup_str: String = telem_mgr.get("_setup_json")
	var setup: Dictionary = JSON.parse_string(setup_str) if not setup_str.is_empty() else {}
	if setup.is_empty():
		_fail("Setup_JSON vacio")
		return
	var assists: Dictionary = setup.get("assists", {})
	print("assists inicial: ", assists)
	assert(assists.has("aids_enabled_mask"), "assists debe exponer aids_enabled_mask")
	assert(bool(assists.get("stability_enabled", false)) == true, "assists.stability_enabled debe ser true al inicio")
	assert(bool(assists.get("enable_stability", false)) == true, "enable_stability debe derivarse de la mascara, no del mirror C++ muerto")
	assert(bool(assists.get("traction_control_enabled", true)) == false, "assists.traction_control_enabled debe ser false al inicio")
	print("[PASS] Telemetria assists fiel al aids_enabled_mask.")

	telem_mgr.call(&"_flush")
	print("=== TODAS LAS VERIFICACIONES DE AYUDAS COMPLETADAS EXITOSAMENTE ===")
	quit(0)
