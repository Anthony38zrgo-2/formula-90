extends Node

const WARMUP_SECONDS := 10.0

var player_vehicle: RigidBody3D
var presentation_core: Node
var measurement_file: FileAccess
var measurement_directory: String
var aerodynamics_backend: String
var suspension_backend: String
var elapsed_seconds: float = 0.0
var measured_ticks: int = 0
var previous_step_count: int = 0
var active: bool = false
var status_label: Label
var maximum_duration_seconds: float = 0.0

func _ready() -> void:
	set_physics_process(false)
	call_deferred("initialize_comparison")

func initialize_comparison() -> void:
	var session := get_parent().get_node("WorldViewport/RaceSession") as RaceSession
	player_vehicle = session.active_vehicle as RigidBody3D
	presentation_core = get_parent().get_node("F90Core")
	if not presentation_core.has_method("get_aerodynamics_comparison_measurements"):
		push_error("Aerodynamics comparison requires a rebuilt native extension")
		get_tree().quit(1)
		return
	aerodynamics_backend = OS.get_environment("FORMULA90_AERODYNAMICS_BACKEND")
	suspension_backend = OS.get_environment("FORMULA90_SUSPENSION_BACKEND")
	measurement_directory = OS.get_environment("FORMULA90_AERODYNAMICS_OUTPUT")
	if measurement_directory.is_empty() or not aerodynamics_backend in ["standard", "simplified"]:
		push_error("Launch aerodynamics comparison through scripts/run_fuji_aerodynamics_comparison.ps1")
		get_tree().quit(1)
		return
	maximum_duration_seconds = float(OS.get_environment("FORMULA90_AERODYNAMICS_DURATION"))
	measurement_file = FileAccess.open(measurement_directory.path_join("physics_samples.csv"), FileAccess.WRITE)
	if measurement_file == null:
		push_error("Cannot write comparison samples")
		get_tree().quit(1)
		return
	measurement_file.store_csv_line(["elapsed_seconds", "vehicle_count", "advancing_vehicles", "moving_vehicles", "contact_sampling_microseconds", "simulation_step_microseconds", "godot_physics_microseconds", "godot_process_microseconds"])
	var overlay := CanvasLayer.new()
	status_label = Label.new()
	status_label.position = Vector2(20.0, 70.0)
	status_label.add_theme_font_size_override("font_size", 22)
	overlay.add_child(status_label)
	get_parent().add_child(overlay)
	active = true
	set_physics_process(true)
	print("PHYSICS_COMPARISON_READY aerodynamics=%s suspension=%s vehicles=1 suspension_rays=12 physics_frequency=%d" % [aerodynamics_backend, suspension_backend, Engine.physics_ticks_per_second])

func _physics_process(duration_seconds: float) -> void:
	if not active:
		return
	elapsed_seconds += duration_seconds
	var measurements: Dictionary = presentation_core.get_aerodynamics_comparison_measurements()
	var step_count := int(measurements.measured_simulation_steps)
	var advancing_vehicle := int(step_count > previous_step_count)
	previous_step_count = step_count
	var contact_microseconds := float(measurements.contact_sampling_microseconds)
	var simulation_microseconds := float(measurements.simulation_step_microseconds)
	var moving_vehicle := int(player_vehicle.linear_velocity.length() > 1.0)
	if elapsed_seconds >= WARMUP_SECONDS:
		measurement_file.store_csv_line([elapsed_seconds, 1, advancing_vehicle, moving_vehicle,
			contact_microseconds, simulation_microseconds, Performance.get_monitor(Performance.TIME_PHYSICS_PROCESS) * 1000000.0,
			Performance.get_monitor(Performance.TIME_PROCESS) * 1000000.0])
		measured_ticks += 1
		if measured_ticks % Engine.physics_ticks_per_second == 0:
			measurement_file.flush()
	status_label.text = "Aero %s | suspension %s | jugador | %d FPS\nContactos %.3f ms | Rust %.3f ms | %.0f s%s" % [
		aerodynamics_backend, suspension_backend, Engine.get_frames_per_second(), contact_microseconds / 1000.0,
		simulation_microseconds / 1000.0, elapsed_seconds,
		" | calentamiento" if elapsed_seconds < WARMUP_SECONDS else " | registrando"]
	if maximum_duration_seconds > 0.0 and elapsed_seconds >= maximum_duration_seconds:
		get_tree().quit()

func _exit_tree() -> void:
	if measurement_file != null:
		measurement_file.flush()
		measurement_file.close()
	if not active:
		return
	var summary := {"backend": aerodynamics_backend, "vehicle_count": 1,
		"suspension_backend": suspension_backend, "suspension_contact_rays": 12,
		"elapsed_seconds": elapsed_seconds, "warmup_seconds": WARMUP_SECONDS, "measured_ticks": measured_ticks,
		"physics_frequency_hertz": Engine.physics_ticks_per_second,
		"human_driving_approval": "pending", "audio_policy": "unchanged player audio"}
	var summary_file := FileAccess.open(measurement_directory.path_join("session_summary.json"), FileAccess.WRITE)
	if summary_file != null:
		summary_file.store_string(JSON.stringify(summary, "\t"))
