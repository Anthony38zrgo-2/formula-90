extends SceneTree

## Regresion de autoridad del ESP en sobreviraje con par a alta velocidad.
##
## Reproduce el caso reportado: 45 m/s, volante a fondo y acelerador a fondo.
## Con la referencia de yaw sin limite (dead-zone) el ESP quedaba inactivo y el
## auto rotaba ~1.5 rad; con el limite por aceleracion lateral debe recortar el
## yaw pico. El escenario tambien fija la mascara ANTES del reset para cubrir la
## preservacion de la mascara runtime en reset_vehicle.

const SCENE_PATH := "res://scenes/vehicles/f1_2030_v10/f1_2030_v10_rust.tscn"
const MASK_BIT_STABILITY := 2
const ENTRY_SPEED_MS := 45.0
const OVERSTEER_FRAMES := 144

func _init() -> void:
	call_deferred("_run")

func _fail(message: String) -> void:
	printerr("[FAIL] ", message)
	quit(1)

func _set_esp(vehicle: Node, enabled: bool) -> void:
	var mask: int = int(vehicle.get("aids_enabled_mask"))
	if enabled:
		mask |= 1 << MASK_BIT_STABILITY
	else:
		mask &= ~(1 << MASK_BIT_STABILITY)
	vehicle.set_aids_enabled_mask(mask)

func _run_phase(vehicle: Node, esp_on: bool) -> Dictionary:
	# The mask is runtime policy: setting it BEFORE the reset also verifies the
	# reset preserves the player's selection (ffi_reset_preserves_runtime_aids_mask).
	_set_esp(vehicle, esp_on)
	vehicle.reset_vehicle(Vector3(0, 0.4, 0), 0.0)
	await physics_frame
	vehicle.linear_velocity = Vector3(0, 0, -ENTRY_SPEED_MS)
	vehicle.angular_velocity = Vector3.ZERO
	vehicle.set_gear_request(4)
	await physics_frame
	var yaw_peak := 0.0
	var heading := 0.0
	for _i in range(OVERSTEER_FRAMES):
		vehicle.set_throttle_amount(1.0)
		vehicle.set_steering_input(1.0)
		await physics_frame
		yaw_peak = maxf(yaw_peak, absf(vehicle.angular_velocity.y))
		heading += vehicle.angular_velocity.y * (1.0 / 120.0)
	return {"yaw_peak": yaw_peak, "heading": heading}

func _run() -> void:
	var ground := StaticBody3D.new()
	var shape := CollisionShape3D.new()
	var box := BoxShape3D.new()
	box.size = Vector3(400.0, 1.0, 4000.0)
	shape.shape = box
	shape.position = Vector3(0, -0.5, -1000)
	ground.add_child(shape)
	root.add_child(ground)

	var packed := load(SCENE_PATH) as PackedScene
	if packed == null:
		_fail("No se pudo cargar %s" % SCENE_PATH)
		return
	var car = packed.instantiate()
	root.add_child(car)
	await physics_frame
	var vehicle = car.get_node_or_null("VehicleRigidBody")
	if vehicle == null:
		_fail("VehicleRigidBody no encontrado")
		return
	vehicle.enable_player_input = false

	print("=== TEST AUTORIDAD ESP (sobreviraje con par a %.0f m/s) ===" % ENTRY_SPEED_MS)
	var off: Dictionary = await _run_phase(vehicle, false)
	print("[ESP=OFF] yaw_peak=%.3f rad/s heading=%.2f rad" % [off["yaw_peak"], off["heading"]])
	var on: Dictionary = await _run_phase(vehicle, true)
	print("[ESP=ON ] yaw_peak=%.3f rad/s heading=%.2f rad" % [on["yaw_peak"], on["heading"]])

	var yaw_off := float(off["yaw_peak"])
	var yaw_on := float(on["yaw_peak"])
	assert(yaw_off > 1.4, "el escenario debe provocar sobreviraje real (off=%.3f)" % yaw_off)
	assert(yaw_on < yaw_off * 0.7, "ESP debe recortar el yaw pico (off=%.3f on=%.3f)" % [yaw_off, yaw_on])
	assert(float(on["heading"]) < float(off["heading"]) * 0.7, "ESP debe reducir la rotacion acumulada")
	print("[PASS] ESP limita el sobreviraje con par: yaw_peak %.3f -> %.3f rad/s" % [yaw_off, yaw_on])
	print("=== TEST AUTORIDAD ESP COMPLETADO EXITOSAMENTE ===")
	quit(0)
