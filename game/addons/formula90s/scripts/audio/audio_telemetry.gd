## AudioTelemetry — optional in-game capture of the Rust audio mix state.
##
## Modo opcional activable ANTES de ejecutar: por defecto está DESACTIVADO
## (enabled = false) y no consume recursos ni escribe archivos. Para activarlo:
##   1) Añade un nodo AudioTelemetry como hijo de F194 (hermano de VehicleAudio)
##      en f1_94.tscn, o añádelo a la escena que lance la sesión.
##   2) En el Inspector pon Enabled = ON antes de pulsar Play.
## Alternativamente, desde código antes de _ready: `audio_telemetry.enabled = true`.
##
## Cuando está activo, muestrea a 20 Hz (LOG_MS=50) el estado expuesto por
## VehicleAudioController (last_*), y escribe un CSV emparejado con su Setup JSON,
## siguiendo el mismo convenio que telemetry_manager.gd (pairing por session_id).
## Para saber exactamente qué está pasando y por qué: RPM, norm, banda dominante,
## pesos, pitches, ganancia, cama de superficie y triggers por frame.

extends Node

## Activar ANTES de ejecutar. Desactivado por defecto: no escribe archivos.
@export var enabled := false

## Referencia al VehicleAudioController del que leer last_*.
@export var audio: Node

const LOG_MS := 50
const AUDIO_COLUMNS := [
	"Time_ms", "RPM", "Norm", "Gear", "Throttle", "Speed_kmh", "Slip", "Surface",
	"W_Idle", "W_Low", "W_Mid", "W_High", "W_Redline",
	"P_Idle", "P_Low", "P_Mid", "P_High", "P_Redline",
	"EngineGain", "BedKey", "BedGain", "Trigger",
	"ActiveBed", "DominantBand",
	"Session_Id"
]

var _file: FileAccess
var _buffer: Array[String] = []
var _prev_sample_ms := 0
var _file_opened := false
var _session_id := ""
var _setup_json := ""

func _ready() -> void:
	_prev_sample_ms = Time.get_ticks_msec()
	if not enabled:
		return
	if audio == null:
		_try_find_audio()
	if audio == null:
		push_warning("[AudioTelemetry] enabled but no VehicleAudioController found; disabling capture.")
		enabled = false
		return
	_open_file()

func _physics_process(_delta: float) -> void:
	if not enabled or audio == null:
		return
	var now := Time.get_ticks_msec()
	if now - _prev_sample_ms < LOG_MS:
		return
	_prev_sample_ms = now
	if not _file_opened:
		_open_file()
	_buffer.append(_format_line(now))
	if _buffer.size() >= 100:
		_flush()

func _exit_tree() -> void:
	if _file_opened:
		_flush()
		if _file:
			_file.close()
		_file_opened = false
	_file = null

func _try_find_audio() -> void:
	var nodes := get_tree().root.find_children("*", "Node", true, false)
	for n in nodes:
		if n.has_method("_band_weights") and n.get("last_weights") != null:
			audio = n
			return
	# Fallback: find by class name via group (if added)
	var grouped := get_tree().get_nodes_in_group("vehicle_audio")
	if grouped.size() > 0:
		audio = grouped[0]

func _open_file() -> void:
	var dir := "res://telemetry/"
	DirAccess.make_dir_recursive_absolute(dir)
	var dt := Time.get_datetime_dict_from_system()
	var msec := Time.get_ticks_msec() % 1000
	var name := "audio_telemetry_%04d%02d%02d_%02d%02d%02d_%03d.csv" % [
		dt.year, dt.month, dt.day, dt.hour, dt.minute, dt.second, msec
	]
	_session_id = name.trim_suffix(".csv")
	_setup_json = JSON.stringify(_build_setup_snapshot(name))
	_file = FileAccess.open(dir + name, FileAccess.WRITE)
	if not _file:
		push_error("[AudioTelemetry] Cannot open file: " + dir + name)
		enabled = false
		return
	_file_opened = true
	_file.store_csv_line(PackedStringArray(AUDIO_COLUMNS))
	# Setup JSON sibling file (pairing)
	var setup_path := dir + _session_id + "_setup.json"
	var setup_file := FileAccess.open(setup_path, FileAccess.WRITE)
	if setup_file:
		setup_file.store_string(_setup_json + "\n")
		setup_file.close()

func _format_line(now_msec: int) -> String:
	var rpm: float = audio.get("last_rpm") if audio.get("last_rpm") != null else 0.0
	var norm: float = audio.get("last_norm") if audio.get("last_norm") != null else 0.0
	var gear: int = 0
	var veh = audio.get("vehicle")
	if veh is NodePath:
		veh = audio.get_node_or_null(veh)
	if veh != null:
		gear = veh.get("current_gear")
	var thr: float = audio.get("last_throttle") if audio.get("last_throttle") != null else 0.0
	var slip: float = audio.get("last_slip") if audio.get("last_slip") != null else 0.0
	var surface: String = audio.get("surface") if audio.get("surface") != null else "asphalt"
	var weights: Array = audio.get("last_weights") if audio.get("last_weights") != null else [0, 0, 0, 0, 0]
	var pitches: Array = audio.get("last_pitches") if audio.get("last_pitches") != null else [1, 1, 1, 1, 1]
	var egain: float = audio.get("last_engine_gain") if audio.get("last_engine_gain") != null else 0.0
	var bed: String = str(audio.get("_active_bed")) if audio.get("_active_bed") != null else ""
	var trigger: String = audio.get("last_trigger") if audio.get("last_trigger") != null else ""
	var speeds: float = audio.get("last_speed_kph") if audio.get("last_speed_kph") != null else 0.0
	# surface bed gain is not exposed; compute from slip/speed if bed active (mirrors controller)
	var bed_gain: float = 0.0
	if bed != "" and bed != null:
		bed_gain = 0.25 + 0.55 * clampf(slip, 0.0, 1.0) + 0.12 * minf(speeds / 120.0, 1.0)
	var dom := 0
	var best := -1.0
	for i in weights.size():
		if float(weights[i]) > best:
			best = float(weights[i])
			dom = i
	return "%d,%.0f,%.4f,%d,%.3f,%.1f,%.3f,%s,%.4f,%.4f,%.4f,%.4f,%.4f,%.4f,%.4f,%.4f,%.4f,%.4f,%.4f,%s,%.4f,%s,%s,%d,%s" % [
		now_msec, rpm, norm, gear, thr, speeds, slip, surface,
		float(weights[0]), float(weights[1]), float(weights[2]), float(weights[3]), float(weights[4]),
		float(pitches[0]), float(pitches[1]), float(pitches[2]), float(pitches[3]), float(pitches[4]),
		egain, bed, bed_gain, trigger, bed, dom, _session_id
	]

func _build_setup_snapshot(filename: String) -> Dictionary:
	return {
		"schema_version": 1,
		"session": {
			"telemetry_file": filename,
			"session_id": _session_id,
			"timestamp_utc": Time.get_datetime_string_from_system(true),
			"physics_hz": Engine.physics_ticks_per_second,
		},
		"audio": {
			"bank": "v10_vehicle",
			"engine_native_rpm": audio.get("ENGINE_BAND_NATIVE_RPM") if audio and audio.get("ENGINE_BAND_NATIVE_RPM") != null else [],
		}
	}

func _flush() -> void:
	if _file and _buffer.size() > 0:
		_file.store_string("\n".join(_buffer) + "\n")
		_buffer.clear()
