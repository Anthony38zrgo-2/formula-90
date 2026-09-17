extends SceneTree

## Regresion de autoridad del ESP contra el "dead-zone" de referencia.
##
## Con volante a fondo a alta velocidad la referencia cinematica
## v*tan(delta)/L supera cualquier capacidad de neumatico (~7 rad/s a 45 m/s).
## Antes del limite por aceleracion lateral el ESP quedaba inactivo en esa
## condicion; este test inyecta un yaw rate con volante a fondo y exige que el
## ESP recorte el yaw y la rotacion acumulada frente a ESP OFF.
##
## El escenario tambien fija la mascara ANTES del reset para cubrir la
## preservacion de la mascara runtime en reset_vehicle.

const SCENE_PATH := "res://scenes/vehicles/f1_2030_v10/f1_2030_v10_rust.tscn"
const MASK_BIT_STABILITY := 2
const ENTRY_SPEED_MS := 45.0
const INJECTED_YAW_RAD_S := 1.5
const RUN_FRAMES := 72

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
	_set_esp(vehicle, esp_on)
	vehicle.reset_vehicle(Vector3(0, 0.4, 0), 0.0)
	await physics_frame
	vehicle.linear_velocity = Vector3(0, 0, -ENTRY_SPEED_MS)
	vehicle.angular_velocity = Vector3(0.0, INJECTED_YAW_RAD_S, 0.0)
	vehicle.set_gear_request(4)
	await physics_frame
	var yaw_peak := 0.0
	var heading := 0.0
	for _i in range(RUN_FRAMES):
		vehicle.set_throttle_amount(1.0)
		vehicle.set_steering_input(1.0)
		await physics_frame
		yaw_peak = maxf(yaw_peak, absf(vehicle.angular_velocity.y))
		heading += vehicle.angular_velocity.y * (1.0 / 120.0)
	return {"yaw_peak": yaw_peak, "yaw_final": absf(vehicle.angular_velocity.y), "heading": heading}

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

	print("=== TEST AUTORIDAD ESP (yaw inyectado con volante a fondo) ===")
	var off: Dictionary = await _run_phase(vehicle, false)
	print("[ESP=OFF] yaw_peak=%.3f yaw_final=%.3f rad/s heading=%.2f rad" % [off["yaw_peak"], off["yaw_final"], off["heading"]])
	var on: Dictionary = await _run_phase(vehicle, true)
	print("[ESP=ON ] yaw_peak=%.3f yaw_final=%.3f rad/s heading=%.2f rad" % [on["yaw_peak"], on["yaw_final"], on["heading"]])

	var yaw_off := float(off["yaw_peak"])
	var yaw_on := float(on["yaw_peak"])
	assert(yaw_off > 0.5, "el escenario debe mantener yaw real (off=%.3f)" % yaw_off)
	assert(float(on["yaw_final"]) < float(off["yaw_final"]) * 0.7,
		"ESP debe amortiguar el yaw residual (off=%.3f on=%.3f)" % [off["yaw_final"], on["yaw_final"]])
	assert(float(on["heading"]) < float(off["heading"]) * 0.7, "ESP debe reducir la rotacion acumulada")
	print("[PASS] ESP limita el yaw con volante a fondo: heading %.2f -> %.2f rad" % [off["heading"], on["heading"]])
	print("=== TEST AUTORIDAD ESP COMPLETADO EXITOSAMENTE ===")
	quit(0)
