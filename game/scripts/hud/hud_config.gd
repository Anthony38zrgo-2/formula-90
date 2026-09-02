class_name HudConfig
extends RefCounted

## Unified, data-driven HUD configuration.
## Loads res://features/hud/config/hud_config.json once and exposes the RetroHud
## section (a RetroHudConfig) together with the full Tyres-panel section
## (TiresSettings). The ArcadeRaceHud injects the retro layout/visibility and the
## tyre settings into their respective panels, so the whole HUD can be tuned by
## editing a single JSON file without touching GDScript.

const DEFAULT_PATH := "res://features/hud/config/hud_config.json"

var retro_hud := RetroHudConfig.new()
var tires := TiresSettings.new()


static func load_from_json(path: String = DEFAULT_PATH) -> HudConfig:
	var config := HudConfig.new()
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		push_warning("HUD config missing: %s. Using defaults." % path)
		return config
	var parsed: Variant = JSON.parse_string(file.get_as_text())
	if not (parsed is Dictionary):
		push_warning("HUD config is invalid: %s. Using defaults." % path)
		return config
	var retro_data: Variant = parsed.get("retro_hud", {})
	if retro_data is Dictionary:
		config.retro_hud.apply(retro_data)
	var tires_data: Variant = parsed.get("tires", {})
	if tires_data is Dictionary:
		config.tires.apply(tires_data)
	return config


func apply_layout_to(target: Node) -> void:
	## Applies the unified retro layout (visibility/scale) to a RetroHudDisplay.
	## Position is intentionally untouched: the embedded RetroHud stays anchored in
	## the scene, so only the JSON-safe scale/visibility knobs are pushed through.
	if target == null or not target.has_method("apply_hud_layout"):
		return
	target.call("apply_hud_layout", retro_hud.display_scale, retro_hud.visible)


class TiresSettings:
	extends RefCounted

	## Compact four-wheel pressure + thermal panel schema (snapshot schema 1).
	## Every visual/layout knob below is editable via hud_config.json.

	var visible := true
	var scale := 0.25
	var size := Vector2(390.0, 170.0)
	var gap := 10.0
	var title := "TYRES"
	var box_size := Vector2(180.0, 60.0)
	var margin_left := 8.0
	var margin_right := 8.0
	var margin_top := 6.0
	var margin_bottom := 6.0
	var h_separation := 10.0
	var v_separation := 6.0
	var wheel_order: Array[String] = ["FL", "FR", "RL", "RR"]

	var cold_max_c := 65.0
	var optimal_max_c := 107.0
	var warm_max_c := 120.0
	var cold_color := Color("73b8ff")
	var optimal_color := Color("a6ff9e")
	var warm_color := Color("ffd16b")
	var hot_color := Color("ff6b61")

	var brake_cold_color := Color("73b8ff")
	var brake_optimal_color := Color("a6ff9e")
	var brake_warm_color := Color("ffd16b")
	var brake_hot_color := Color("ff6b61")
	var brake_critical_color := Color("d91f1f")


	func apply(data: Dictionary) -> void:
		visible = bool(data.get("visible", visible))
		scale = maxf(float(data.get("scale", scale)), 0.05)
		size = _vec2(data.get("size", [size.x, size.y]), size)
		gap = maxf(float(data.get("gap", gap)), 0.0)
		title = str(data.get("title", title))
		box_size = _vec2(data.get("box_size", [box_size.x, box_size.y]), box_size)
		margin_left = maxf(float(data.get("margin_left", margin_left)), 0.0)
		margin_right = maxf(float(data.get("margin_right", margin_right)), 0.0)
		margin_top = maxf(float(data.get("margin_top", margin_top)), 0.0)
		margin_bottom = maxf(float(data.get("margin_bottom", margin_bottom)), 0.0)
		h_separation = maxf(float(data.get("h_separation", h_separation)), 0.0)
		v_separation = maxf(float(data.get("v_separation", v_separation)), 0.0)

		var order: Array = data.get("wheel_order", wheel_order.duplicate())
		if order is Array and not order.is_empty():
			var cleaned: Array[String] = []
			for entry in order:
				cleaned.append(str(entry).to_upper())
			wheel_order = cleaned

		var temp: Variant = data.get("temperature", {})
		if temp is Dictionary:
			cold_max_c = maxf(float(temp.get("cold_max_c", cold_max_c)), 0.0)
			optimal_max_c = maxf(float(temp.get("optimal_max_c", optimal_max_c)), cold_max_c)
			warm_max_c = maxf(float(temp.get("warm_max_c", warm_max_c)), optimal_max_c)
			cold_color = _color(temp.get("cold_color", "#73b8ff"), cold_color)
			optimal_color = _color(temp.get("optimal_color", "#a6ff9e"), optimal_color)
			warm_color = _color(temp.get("warm_color", "#ffd16b"), warm_color)
			hot_color = _color(temp.get("hot_color", "#ff6b61"), hot_color)

		var brake: Variant = data.get("brake", {})
		if brake is Dictionary:
			brake_cold_color = _color(brake.get("cold_color", "#73b8ff"), brake_cold_color)
			brake_optimal_color = _color(brake.get("optimal_color", "#a6ff9e"), brake_optimal_color)
			brake_warm_color = _color(brake.get("warm_color", "#ffd16b"), brake_warm_color)
			brake_hot_color = _color(brake.get("hot_color", "#ff6b61"), brake_hot_color)
			brake_critical_color = _color(brake.get("critical_color", "#d91f1f"), brake_critical_color)


	func to_dict() -> Dictionary:
		return {
			"visible": visible,
			"scale": scale,
			"size": [size.x, size.y],
			"gap": gap,
			"title": title,
			"box_size": [box_size.x, box_size.y],
			"margin_left": margin_left,
			"margin_right": margin_right,
			"margin_top": margin_top,
			"margin_bottom": margin_bottom,
			"h_separation": h_separation,
			"v_separation": v_separation,
			"wheel_order": wheel_order.duplicate(),
			"temperature": {
				"cold_max_c": cold_max_c,
				"optimal_max_c": optimal_max_c,
				"warm_max_c": warm_max_c,
				"cold_color": cold_color.to_html(true),
				"optimal_color": optimal_color.to_html(true),
				"warm_color": warm_color.to_html(true),
				"hot_color": hot_color.to_html(true)
			},
			"brake": {
				"cold_color": brake_cold_color.to_html(true),
				"optimal_color": brake_optimal_color.to_html(true),
				"warm_color": brake_warm_color.to_html(true),
				"hot_color": brake_hot_color.to_html(true),
				"critical_color": brake_critical_color.to_html(true)
			}
		}


	func _vec2(value: Variant, fallback: Vector2) -> Vector2:
		if value is Array and value.size() >= 2:
			return Vector2(float(value[0]), float(value[1]))
		return fallback


	func _color(value: Variant, fallback: Color) -> Color:
		if value is String and Color.html_is_valid(value):
			return Color(value)
		return fallback
