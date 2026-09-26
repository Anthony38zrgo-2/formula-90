extends SceneTree

const SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"
const START_FINISH_POSITION := Vector3(-294.278, 0.095, -275.19)
const START_FINISH_FORWARD := Vector3(-0.0069, 0.0, -0.99998)
const FULL_THROTTLE_FRAMES := 1200
const STEADY_WINDOW_FRAMES := 600
const LAP_DISTANCE_STEPS := 110
const LAP_DISTANCE_STEP_M := 20.0
const MINIMUM_BURNED_MASS_KG := 0.05

func _init() -> void:
	call_deferred("_run")

func _fail(message: String, failures: Array[String]) -> void:
	printerr("[FAIL] " + message)
	failures.append(message)

func _run() -> void:
	var failures: Array[String] = []
	var packed := load(SCENE_PATH) as PackedScene
	if packed == null:
		_fail("No se pudo cargar %s" % SCENE_PATH, failures)
		quit(1)
		return

	var compositor := packed.instantiate()
	root.add_child(compositor)
	for _frame in 8:
		await process_frame

	var vehicle := compositor.find_child("VehicleRigidBody", true, false) as RigidBody3D
	var race_session := compositor.find_child("RaceSession", true, false)
	if vehicle == null or race_session == null:
		_fail("Vehiculo o RaceSession ausentes.", failures)
		compositor.queue_free()
		quit(1)
		return

	var lap_timing = race_session.get("lap_timing")
	if lap_timing == null or not bool(lap_timing.get("is_configured")):
		_fail("LapTimingController no configurado.", failures)

	var fuel_before: Dictionary = vehicle.call("get_fuel_state_snapshot")
	var remaining_before := float(fuel_before.get("remaining_kg", -1.0))
	var capacity_kg := float(fuel_before.get("capacity_kg", 0.0))
	if capacity_kg != 110.0 or remaining_before <= 0.0 or remaining_before > 7.6:
		_fail("Carga inicial invalida: %s" % [fuel_before], failures)

	for _settle_frame in 120:
		await physics_frame

	remaining_before = float(vehicle.call("get_fuel_state_snapshot").get("remaining_kg", -1.0))
	print("[AUDIT] fuel_before=%.4f kg capacity=%.1f kg mass=%.2f kg" % [remaining_before, capacity_kg, vehicle.mass])

	vehicle.set("enable_player_input", false)
	vehicle.set("throttle_amount", 1.0)
	var physics_step_seconds := 1.0 / float(Engine.physics_ticks_per_second)
	var expected_burn_kg := 0.0
	var remaining_at_steady_window_start := -1.0
	for _frame in FULL_THROTTLE_FRAMES:
		vehicle.set("throttle_amount", 1.0)
		await physics_frame
		var thermal: Dictionary = vehicle.call("get_engine_thermal_state_snapshot")
		expected_burn_kg += (
			float(thermal.get("engine_mechanical_power_w", 0.0)) / 1000.0 * 0.30 / 3600.0
			+ 2.0 / 3600.0) * physics_step_seconds
		if _frame == FULL_THROTTLE_FRAMES - STEADY_WINDOW_FRAMES:
			remaining_at_steady_window_start = float(
				vehicle.call("get_fuel_state_snapshot").get("remaining_kg", -1.0))
		if (_frame + 1) % 120 == 0:
			print("[AUDIT] t=%.1fs fuel=%.4f kg mech_power=%.1f kW expected_burn=%.4f kg rpm=%.0f speed=%.1f" % [
				float(_frame + 1) / 60.0,
				float(vehicle.call("get_fuel_state_snapshot").get("remaining_kg", -1.0)),
				float(thermal.get("engine_mechanical_power_w", 0.0)) / 1000.0,
				expected_burn_kg,
				float(vehicle.get("motor_rpm")),
				float(vehicle.get("speed_kmh"))])
	print("[AUDIT] full_throttle_speed=%.1f km/h rpm=%.0f gear=%d" % [
		float(vehicle.get("speed_kmh")),
		float(vehicle.get("motor_rpm")),
		int(vehicle.get("current_gear"))])

	var fuel_after: Dictionary = vehicle.call("get_fuel_state_snapshot")
	var remaining_after := float(fuel_after.get("remaining_kg", -1.0))
	var burned_kg := remaining_before - remaining_after
	var mass_after := vehicle.mass
	if burned_kg < MINIMUM_BURNED_MASS_KG:
		_fail("El combustible no bajo con carga: %.4f kg" % burned_kg, failures)
	if absf(burned_kg - expected_burn_kg) > 0.001:
		_fail("Consumo real %.4f kg != flujo de potencia esperado %.4f kg" % [burned_kg, expected_burn_kg], failures)
	if absf(mass_after - (600.0 + remaining_after)) > 0.2:
		_fail("La masa del RigidBody no refleja el combustible restante.", failures)
	print("[AUDIT] burned=%.4f kg over %d frames | remaining=%.4f kg | mass=%.2f kg" % [
		burned_kg, FULL_THROTTLE_FRAMES, remaining_after, mass_after])

	vehicle.set("throttle_amount", 0.0)
	vehicle.freeze = true
	var start_finish_forward := START_FINISH_FORWARD.normalized()
	vehicle.global_position = START_FINISH_POSITION - start_finish_forward * 8.0
	await physics_frame
	vehicle.global_position = START_FINISH_POSITION + start_finish_forward * 8.0
	await physics_frame
	if int(lap_timing.get("current_lap_number")) != 1:
		_fail("El primer cruce de meta no inicio la vuelta 1.", failures)

	for _step in LAP_DISTANCE_STEPS:
		vehicle.global_position += start_finish_forward * LAP_DISTANCE_STEP_M
		await physics_frame
	vehicle.global_position = START_FINISH_POSITION - start_finish_forward * 8.0
	await physics_frame
	vehicle.global_position = START_FINISH_POSITION + start_finish_forward * 8.0
	await physics_frame

	var lap_count := int(lap_timing.get("lap_count"))
	var last_lap_time := float(lap_timing.get("last_lap_time_seconds"))
	var best_lap_time := float(lap_timing.get("best_lap_time_seconds"))
	if lap_count != 1:
		_fail("Se esperaba una vuelta completada, got %d." % lap_count, failures)
	if last_lap_time <= 0.0 or not is_equal_approx(last_lap_time, best_lap_time):
		_fail("Tiempo de vuelta invalido last=%.3f best=%.3f" % [last_lap_time, best_lap_time], failures)
	print("[AUDIT] laps=%d last=%s best=%s" % [
		lap_count,
		lap_timing.call("format_lap_time", last_lap_time),
		lap_timing.call("format_lap_time", best_lap_time)])

	for _hud_frame in 3:
		await process_frame

	var engine_panel := compositor.get_node_or_null(
		"DisplayAspect/DisplayStage/HudLayer/DebugHud/EngineTemperaturePanel") as Node
	var lap_panel := compositor.get_node_or_null(
		"DisplayAspect/DisplayStage/HudLayer/DebugHud/LapTimingPanel") as Node
	var fuel_label := engine_panel.get("_fuel_label") as Label if engine_panel != null else null
	var current_lap_label := lap_panel.get("_lap_value_label") as Label if lap_panel != null else null
	var last_lap_label := lap_panel.get("_last_lap_value_label") as Label if lap_panel != null else null
	var best_lap_label := lap_panel.get("_best_lap_value_label") as Label if lap_panel != null else null
	var engine_text := fuel_label.text if fuel_label != null else ""
	var current_lap_text := current_lap_label.text if current_lap_label != null else ""
	if not engine_text.begins_with("FUEL"):
		_fail("El panel ENGINE no muestra FUEL.", failures)
	if current_lap_text != "2":
		_fail("El panel LAP no muestra la vuelta actual.", failures)
	print("[AUDIT] ENGINE=%s | LAP=%s | LAST=%s | BEST=%s" % [
		engine_text,
		current_lap_text,
		last_lap_label.text if last_lap_label != null else "",
		best_lap_label.text if best_lap_label != null else ""])

	var steady_window_seconds := float(STEADY_WINDOW_FRAMES) * physics_step_seconds
	var steady_burn_kg := remaining_at_steady_window_start - remaining_after
	var steady_burn_rate_kg_per_second := steady_burn_kg / maxf(steady_window_seconds, 0.001)
	var representative_lap_seconds := 90.0
	var representative_duty := 0.55
	var kilograms_per_lap_estimate := (
		steady_burn_rate_kg_per_second * representative_duty * representative_lap_seconds)
	var configured_laps_estimate := remaining_before / maxf(kilograms_per_lap_estimate, 0.001)
	if configured_laps_estimate < 2.0 or configured_laps_estimate > 4.0:
		_fail("La carga inicial no corresponde a ~3 vueltas de Fuji: %.2f" % configured_laps_estimate, failures)
	print("[AUDIT] steady_burn_rate=%.5f kg/s (last %.0f s) => Fuji lap estimate %.3f kg/lap (%.0f s at %.0f%% duty) => %.1f laps on %.1f kg" % [
		steady_burn_rate_kg_per_second,
		steady_window_seconds,
		kilograms_per_lap_estimate,
		representative_lap_seconds,
		representative_duty * 100.0,
		capacity_kg / maxf(kilograms_per_lap_estimate, 0.001),
		capacity_kg])
	print("[AUDIT] configured initial load %.3f kg corresponds to %.2f estimated Fuji laps" % [
		remaining_before,
		configured_laps_estimate])

	compositor.queue_free()
	if failures.is_empty():
		print("[PASS] Fuel burn, mass coupling, lap counting, lap times and HUD panels verified on Fuji.")
	quit(failures.size())