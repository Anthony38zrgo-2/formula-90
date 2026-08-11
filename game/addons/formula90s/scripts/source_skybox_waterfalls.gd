extends Node

## Advances the small waterfall atlas on the distant mountain cards. The cards
## stay in the regular 3D depth buffer; only their texture frame changes.

const WATERFALL_GROUP := &"source_skybox_waterfalls"

@export_range(1.0, 30.0, 0.5, "suffix: fps") var frames_per_second := 8.0

var _elapsed := 0.0
var _frame := -1


func _ready() -> void:
	_apply_frame(0)


func _process(delta: float) -> void:
	_elapsed += delta
	var next_frame := int(floor(_elapsed * frames_per_second)) % 4
	if next_frame != _frame:
		_apply_frame(next_frame)


func _apply_frame(next_frame: int) -> void:
	_frame = next_frame
	for candidate in get_tree().get_nodes_in_group(WATERFALL_GROUP):
		var waterfall := candidate as Sprite3D
		if waterfall == null:
			continue
		var frame_count := maxi(1, waterfall.hframes)
		waterfall.frame = (_frame + waterfall.get_index()) % frame_count
