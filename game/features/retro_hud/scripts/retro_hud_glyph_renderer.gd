class_name RetroHudGlyphRenderer
extends Control

## Hand-built segmented glyphs. Coordinates are normalized in a 1 x 1 cell.
const STROKES := {
	"0": [[0.16, 0.08, 0.84, 0.08], [0.84, 0.08, 0.84, 0.92], [0.84, 0.92, 0.16, 0.92], [0.16, 0.92, 0.16, 0.08]],
	"1": [[0.50, 0.08, 0.50, 0.92], [0.30, 0.92, 0.72, 0.92]],
	"2": [[0.16, 0.10, 0.84, 0.10], [0.84, 0.10, 0.84, 0.48], [0.84, 0.48, 0.16, 0.48], [0.16, 0.48, 0.16, 0.90], [0.16, 0.90, 0.84, 0.90]],
	"3": [[0.16, 0.10, 0.84, 0.10], [0.84, 0.10, 0.84, 0.90], [0.16, 0.50, 0.78, 0.50], [0.16, 0.90, 0.84, 0.90]],
	"4": [[0.16, 0.10, 0.16, 0.50], [0.16, 0.50, 0.84, 0.50], [0.84, 0.10, 0.84, 0.92]],
	"5": [[0.84, 0.10, 0.16, 0.10], [0.16, 0.10, 0.16, 0.50], [0.16, 0.50, 0.84, 0.50], [0.84, 0.50, 0.84, 0.90], [0.84, 0.90, 0.16, 0.90]],
	"6": [[0.84, 0.10, 0.16, 0.10], [0.16, 0.10, 0.16, 0.90], [0.16, 0.50, 0.84, 0.50], [0.84, 0.50, 0.84, 0.90], [0.84, 0.90, 0.16, 0.90]],
	"7": [[0.16, 0.10, 0.84, 0.10], [0.84, 0.10, 0.42, 0.92]],
	"8": [[0.16, 0.08, 0.84, 0.08], [0.16, 0.08, 0.16, 0.92], [0.84, 0.08, 0.84, 0.92], [0.16, 0.50, 0.84, 0.50], [0.16, 0.92, 0.84, 0.92]],
	"9": [[0.16, 0.10, 0.84, 0.10], [0.16, 0.10, 0.16, 0.50], [0.16, 0.50, 0.84, 0.50], [0.84, 0.10, 0.84, 0.90], [0.16, 0.90, 0.84, 0.90]],
	"R": [[0.16, 0.08, 0.16, 0.92], [0.16, 0.08, 0.78, 0.08], [0.78, 0.08, 0.78, 0.48], [0.16, 0.48, 0.78, 0.48], [0.48, 0.48, 0.86, 0.92]],
	"P": [[0.16, 0.08, 0.16, 0.92], [0.16, 0.08, 0.82, 0.08], [0.82, 0.08, 0.82, 0.48], [0.16, 0.48, 0.82, 0.48]],
	"M": [[0.14, 0.92, 0.14, 0.08], [0.14, 0.08, 0.50, 0.50], [0.50, 0.50, 0.86, 0.08], [0.86, 0.08, 0.86, 0.92]],
	"N": [[0.16, 0.92, 0.16, 0.08], [0.16, 0.08, 0.84, 0.92], [0.84, 0.92, 0.84, 0.08]],
	"K": [[0.16, 0.08, 0.16, 0.92], [0.84, 0.08, 0.16, 0.50], [0.16, 0.50, 0.84, 0.92]],
	"H": [[0.16, 0.08, 0.16, 0.92], [0.84, 0.08, 0.84, 0.92], [0.16, 0.50, 0.84, 0.50]],
	"G": [[0.84, 0.16, 0.66, 0.08], [0.66, 0.08, 0.16, 0.08], [0.16, 0.08, 0.16, 0.92], [0.16, 0.92, 0.84, 0.92], [0.84, 0.92, 0.84, 0.50], [0.84, 0.50, 0.50, 0.50]],
	"E": [[0.84, 0.08, 0.16, 0.08], [0.16, 0.08, 0.16, 0.92], [0.16, 0.50, 0.72, 0.50], [0.16, 0.92, 0.84, 0.92]],
	"A": [[0.16, 0.92, 0.16, 0.08], [0.16, 0.08, 0.84, 0.08], [0.84, 0.08, 0.84, 0.92], [0.16, 0.50, 0.84, 0.50]],
	"X": [[0.16, 0.08, 0.84, 0.92], [0.84, 0.08, 0.16, 0.92]],
	"/": [[0.18, 0.92, 0.82, 0.08]],
	".": [[0.46, 0.88, 0.54, 0.88]],
	":": [[0.46, 0.28, 0.54, 0.28], [0.46, 0.76, 0.54, 0.76]],
	"%": [[0.16, 0.08, 0.84, 0.92], [0.18, 0.20, 0.28, 0.20], [0.72, 0.80, 0.82, 0.80]],
	"-": [[0.18, 0.50, 0.82, 0.50]]
}

@export_multiline var text := ""
@export var glyph_color := Color("061414")
@export_range(4.0, 256.0, 1.0) var glyph_height := 32.0
@export_range(0.0, 1.0, 0.01) var tracking := 0.16
@export_range(0.02, 0.25, 0.01) var stroke_ratio := 0.10
@export_enum("left", "center", "right") var alignment := "center"


func set_text(next_text: String) -> void:
	if text == next_text:
		return
	text = next_text
	queue_redraw()


func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		queue_redraw()


func _draw() -> void:
	var width := _text_width(text)
	var start_x := 0.0
	if alignment == "center":
		start_x = (size.x - width) * 0.5
	elif alignment == "right":
		start_x = size.x - width
	var y := (size.y - glyph_height) * 0.5
	var cell_width := glyph_height * 0.72
	for character in text.to_upper():
		_draw_glyph(character, Vector2(start_x, y), cell_width, glyph_height)
		start_x += cell_width + glyph_height * tracking


func _text_width(value: String) -> float:
	if value.is_empty():
		return 0.0
	return value.length() * glyph_height * 0.72 + (value.length() - 1) * glyph_height * tracking


func _draw_glyph(character: String, origin: Vector2, width: float, height: float) -> void:
	var strokes: Array = STROKES.get(character, [])
	var thickness := maxf(1.0, height * stroke_ratio)
	for stroke in strokes:
		var from := origin + Vector2(float(stroke[0]) * width, float(stroke[1]) * height)
		var to := origin + Vector2(float(stroke[2]) * width, float(stroke[3]) * height)
		draw_line(from, to, glyph_color, thickness, true)
