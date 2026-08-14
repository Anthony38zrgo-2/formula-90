# Retro HUD

`RetroHudDisplay` is presentation-only. It receives a `RetroHudState` or calls to
`set_readout(speed_kph, rpm, gear_label)` and never resolves vehicle nodes.

The race HUD is the adapter that reads the existing vehicle properties and forwards
them to this component. Other game modes can instead provide their own state source.

Visual tuning lives in [config/retro_hud.json](config/retro_hud.json). Call
`reload_config()` on an existing display after changing the file to apply it during
runtime. The base panel is a generated original asset in `assets/`; all variable
labels and digits are drawn by the manual stroke glyph renderer.

Open `scenes/retro_hud_demo.tscn` for the standalone mock-driven preview.
