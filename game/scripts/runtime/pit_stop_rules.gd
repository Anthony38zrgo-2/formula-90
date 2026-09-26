class_name PitStopRules
extends RefCounted

## Data-driven pit-stop service rules. Loaded once by the PitStopController so
## the tire-change duration, refuel flow and stop thresholds live in JSON
## instead of gameplay code. Fuel targets are always expressed in laps and
## converted with the active vehicle profile's estimated consumption.

const DEFAULT_PATH := "res://data/pit_stop/pit_stop_rules.json"

var tire_change_seconds := 3.0
var refuel_rate_kg_per_s := 5.0
var stop_speed_threshold_kmh := 2.0
var stop_hold_seconds := 0.25
var default_fuel_laps := 15
var minimum_fuel_laps := 1
var compound_labels: Array[String] = ["BLANDOS"]


static func load_from_json(path: String = DEFAULT_PATH) -> PitStopRules:
	var rules := PitStopRules.new()
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		push_warning("PitStopRules: no se pudo abrir %s; se usan los valores por defecto." % path)
		return rules
	var parsed: Variant = JSON.parse_string(file.get_as_text())
	if not (parsed is Dictionary):
		push_warning("PitStopRules: JSON invalido en %s; se usan los valores por defecto." % path)
		return rules
	var data: Dictionary = parsed
	rules.tire_change_seconds = maxf(float(data.get("tire_change_seconds", rules.tire_change_seconds)), 0.0)
	rules.refuel_rate_kg_per_s = maxf(float(data.get("refuel_rate_kg_per_s", rules.refuel_rate_kg_per_s)), 0.0001)
	rules.stop_speed_threshold_kmh = maxf(float(data.get("stop_speed_threshold_kmh", rules.stop_speed_threshold_kmh)), 0.0)
	rules.stop_hold_seconds = maxf(float(data.get("stop_hold_seconds", rules.stop_hold_seconds)), 0.0)
	rules.default_fuel_laps = maxi(int(data.get("default_fuel_laps", rules.default_fuel_laps)), 1)
	rules.minimum_fuel_laps = maxi(int(data.get("minimum_fuel_laps", rules.minimum_fuel_laps)), 1)
	var compounds: Variant = data.get("tire_compounds", [])
	if compounds is Array and not (compounds as Array).is_empty():
		var labels: Array[String] = []
		for raw_label in compounds:
			var label := str(raw_label).strip_edges()
			if not label.is_empty():
				labels.append(label)
		if not labels.is_empty():
			rules.compound_labels = labels
	return rules
