@tool
extends SceneTree

func _init():
	print("[SMOKE TEST] Iniciando pruebas del vehiculo F1 1996...")
	var failures = 0
	
	# 1. Cargar mallas GLB
	var body_scene = load("res://assets/models/vehicles/f1_1996/f1_body.glb")
	if not body_scene or not body_scene.instantiate():
		printerr("[FAIL] f1_body.glb no pudo cargarse o instanciarse.")
		failures += 1
	else:
		print("[PASS] f1_body.glb cargado correctamente.")

	var wheel_scene = load("res://assets/models/vehicles/f1_1996/wheel_fl.glb")
	if not wheel_scene or not wheel_scene.instantiate():
		printerr("[FAIL] wheel_fl.glb no pudo cargarse o instanciarse.")
		failures += 1
	else:
		print("[PASS] wheel_fl.glb cargado correctamente.")

	# 2. Cargar escena del vehiculo
	var car_scene = load("res://scenes/vehicles/f1_1996_car.tscn")
	if not car_scene:
		printerr("[FAIL] f1_1996_car.tscn no se encontro.")
		failures += 1
		quit(failures)
		return
		
	var car = car_scene.instantiate()
	if not car:
		printerr("[FAIL] No se pudo instanciar f1_1996_car.tscn.")
		failures += 1
		quit(failures)
		return
	print("[PASS] f1_1996_car.tscn instanciado.")

	var car_2026_scene = load("res://scenes/vehicles/f1_2026_car.tscn")
	if not car_2026_scene or not car_2026_scene.instantiate():
		printerr("[FAIL] f1_2026_car.tscn no pudo cargarse o instanciarse.")
		failures += 1
	else:
		print("[PASS] f1_2026_car.tscn instanciado correctamente.")

	# 3. Validar propiedades de transmision y masa
	if car.mass != 595.0:
		printerr("[FAIL] Masa incorrecta: ", car.mass, " (esperado 595.0)")
		failures += 1
	else:
		print("[PASS] Masa del monoplaza: 595 kg")

	if not car.gear_ratios or car.gear_ratios.size() == 0:
		printerr("[FAIL] gear_ratios vacio o invalido!")
		failures += 1
	else:
		print("[PASS] gear_ratios validos: ", car.gear_ratios)

	# 4. Validar torque y calculos de motor
	var torque_5000 = car.get_torque_at_rpm(5000.0) if car.has_method("get_torque_at_rpm") else 0.0
	if is_nan(torque_5000) or is_inf(torque_5000) or torque_5000 <= 0.0:
		printerr("[FAIL] Torque en 5000 RPM es NaN/Inf o invalido: ", torque_5000)
		failures += 1
	else:
		print("[PASS] Torque a 5000 RPM: ", roundf(torque_5000), " N.m")

	var torque_15000 = car.get_torque_at_rpm(15000.0) if car.has_method("get_torque_at_rpm") else 0.0
	if is_nan(torque_15000) or is_inf(torque_15000) or torque_15000 <= 0.0:
		printerr("[FAIL] Torque en 15000 RPM es NaN/Inf o invalido: ", torque_15000)
		failures += 1
	else:
		print("[PASS] Torque a 15000 RPM: ", roundf(torque_15000), " N.m")

	# 5. Resultado final
	if failures == 0:
		print("\n[TODOS LOS SMOKE TESTS PASARON EXITOSAMENTE]")
		quit(0)
	else:
		printerr("\n[SMOKE TEST FALLO CON ", failures, " ERRORES]")
		quit(failures)
