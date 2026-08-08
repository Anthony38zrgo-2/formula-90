extends CheckButton

func _ready():
	toggled.connect(func(pressed: bool):
		TelemetryManager.enabled = pressed
	)
