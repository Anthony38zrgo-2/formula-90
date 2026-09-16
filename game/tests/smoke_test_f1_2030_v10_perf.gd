extends SceneTree
## SUS-GEO-11: runtime performance smoke for the F1 2030 V10 geometric profile.
##
## Loads the canonical Fuji runtime session (same scene as run_f1_94.ps1),
## settles the car, then measures Godot's physics-process time and frame rate
## while idle and while driving. The physics budget at 120 Hz is 8.33 ms.
##
## Usage:
##   godot --headless --path game --script res://tests/smoke_test_f1_2030_v10_perf.gd

const DEFAULT_SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"
const WARMUP_FRAMES := 360
const IDLE_FRAMES := 600
const DRIVE_FRAMES := 600
const BUDGET_MS := 1000.0 / 120.0

func _init() -> void:
	call_deferred("_run")

func _argument(prefix: String, fallback: String) -> String:
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with(prefix):
			return argument.trim_prefix(prefix)
	return fallback

## SUS-GEO-11 attribution: set `--no-visual-suspension` to measure the physics
## DLL without the GDScript linkage solver (visual controller) in the frame.
func _disable_visual_suspension(node: Node) -> int:
	var disabled := 0
	for child in node.get_children():
		if child.get("enable_suspension_geometry") != null:
			child.set("enable_suspension_geometry", false)
			disabled += 1
		disabled += _disable_visual_suspension(child)
	return disabled

func _stats(samples_ms: Array, samples_fps: Array) -> Dictionary:
	var sorted_ms := samples_ms.duplicate()
	sorted_ms.sort()
	var avg_ms := 0.0
	for value in samples_ms:
		avg_ms += value
	avg_ms /= maxf(float(samples_ms.size()), 1.0)
	var avg_fps := 0.0
	for value in samples_fps:
		avg_fps += value
	avg_fps /= maxf(float(samples_fps.size()), 1.0)
	var p95_index: int = clampi(int(floor(float(sorted_ms.size()) * 0.95)), 0, sorted_ms.size() - 1)
	var p50_index: int = clampi(int(floor(float(sorted_ms.size()) * 0.50)), 0, sorted_ms.size() - 1)
	return {
		"avg_ms": avg_ms,
		"p50_ms": sorted_ms[p50_index],
		"p95_ms": sorted_ms[p95_index],
		"max_ms": sorted_ms[sorted_ms.size() - 1],
		"avg_fps": avg_fps,
	}

func _measure(vehicle: Node, frames: int, driving: bool) -> Dictionary:
	var physics_ms: Array = []
	var wall_ms: Array = []
	var fps: Array = []
	for _frame in frames:
		if driving and vehicle != null and vehicle.get("throttle_amount") != null:
			vehicle.set("throttle_amount", 1.0)
		var before_usec := Time.get_ticks_usec()
		await physics_frame
		wall_ms.append(float(Time.get_ticks_usec() - before_usec) / 1000.0)
		physics_ms.append(Performance.get_monitor(Performance.TIME_PHYSICS_PROCESS) * 1000.0)
		fps.append(Engine.get_frames_per_second())
	var out := _stats(physics_ms, fps)
	var wall := _stats(wall_ms, fps)
	out["wall_avg_ms"] = wall["avg_ms"]
	out["wall_p95_ms"] = wall["p95_ms"]
	out["wall_max_ms"] = wall["max_ms"]
	return out

func _run() -> void:
	var failures: Array[String] = []
	var scene_path := _argument("--scene=", DEFAULT_SCENE_PATH)
	var packed := load(scene_path) as PackedScene
	if packed == null:
		printerr("[FAIL] Perf smoke could not load %s" % scene_path)
		quit(1)
		return
	var compositor := packed.instantiate()
	root.add_child(compositor)
	for _frame in 8:
		await process_frame

	var vehicle := compositor.find_child("VehicleRigidBody", true, false)
	if vehicle == null:
		printerr("[FAIL] Perf smoke could not find VehicleRigidBody")
		quit(1)
		return
	vehicle.set("enable_player_input", false)
	var no_visual := _argument("--no-visual-suspension", "false") == "true"
	if no_visual:
		var disabled := _disable_visual_suspension(compositor)
		print("[PERF] visual suspension controllers disabled: %d" % disabled)

	for _frame in WARMUP_FRAMES:
		await physics_frame

	var phase_frames := int(_argument("--frames=", str(IDLE_FRAMES)))
	var idle := await _measure(vehicle, phase_frames, false)
	var drive := await _measure(vehicle, phase_frames, true)
	print("[PERF] idle monitor_ms avg=%.3f p50=%.3f p95=%.3f max=%.3f | tick_ms avg=%.3f p95=%.3f max=%.3f | fps=%.1f" % [
		idle["avg_ms"], idle["p50_ms"], idle["p95_ms"], idle["max_ms"],
		idle["wall_avg_ms"], idle["wall_p95_ms"], idle["wall_max_ms"], idle["avg_fps"],
	])
	print("[PERF] drive monitor_ms avg=%.3f p50=%.3f p95=%.3f max=%.3f | tick_ms avg=%.3f p95=%.3f max=%.3f | fps=%.1f" % [
		drive["avg_ms"], drive["p50_ms"], drive["p95_ms"], drive["max_ms"],
		drive["wall_avg_ms"], drive["wall_p95_ms"], drive["wall_max_ms"], drive["avg_fps"],
	])
	print("[PERF] budget_ms=%.3f idle_monitor_avg=%.3f drive_monitor_avg=%.3f" % [
		BUDGET_MS, idle["avg_ms"], drive["avg_ms"],
	])

	# Hard gate: the runtime must sustain the 120 Hz tick pacing (wall time
	# between physics_frame signals). The physics monitor is reported for the
	# record but is noisy in headless debug runs. A stalled solver (SUS-GEO-11
	# regression) pushes the wall tick to ~100 ms and fails this gate.
	var wall_limit_ms := BUDGET_MS * 1.15
	if idle["wall_avg_ms"] > wall_limit_ms:
		failures.append("idle tick avg %.3f ms exceeds 120 Hz pacing (limit %.3f)" % [idle["wall_avg_ms"], wall_limit_ms])
	if drive["wall_avg_ms"] > wall_limit_ms:
		failures.append("driving tick avg %.3f ms exceeds 120 Hz pacing (limit %.3f)" % [drive["wall_avg_ms"], wall_limit_ms])

	compositor.queue_free()
	if failures.is_empty():
		print("[PASS] F1 2030 V10 runtime stays inside the 120 Hz physics budget.")
		quit(0)
	else:
		for failure in failures:
			printerr("[FAIL] " + failure)
		quit(failures.size())
