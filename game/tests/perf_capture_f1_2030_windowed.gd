extends SceneTree

## SUS-GEO track: windowed rendered-frame capture for the F1 2030 runtime.
##
## Measures per-frame wall deltas (rendered frame time), physics/process
## monitors and render counters with p50/p95/p99/max percentiles. VSync is
## disabled during the capture so frame deltas expose true render cost instead
## of clamping to the monitor refresh. The window, resolution, camera, vehicle
## and track stay fixed (the canonical vehicle_test_session scene).
##
## Usage:
##   godot --path game --script res://tests/perf_capture_f1_2030_windowed.gd -- \
##     --frames=600 [--warmup=240] [--throttle=0.0] [--out=res://..]

const DEFAULT_SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"
const WARMUP_FRAMES := 240
const MEASURE_FRAMES := 600

func _init() -> void:
	call_deferred("_run")

func _argument(key: String, fallback: String) -> String:
	var prefix := key + "="
	for argument in OS.get_cmdline_user_args():
		if argument == key:
			return ""
		if argument.begins_with(prefix):
			return argument.trim_prefix(prefix)
	return fallback

## Boolean CLI flag: `--key` and `--key=true|1` enable; absent or
## `--key=false|0` disables.
func _flag(key: String) -> bool:
	for argument in OS.get_cmdline_user_args():
		if argument == key:
			return true
		if argument.begins_with(key + "="):
			var value := argument.trim_prefix(key + "=").to_lower()
			return value == "true" or value == "1"
	return false

func _linkage_solve_count(node: Node) -> int:
	var total := 0
	if node.get("linkage_solve_count") != null:
		total += int(node.get("linkage_solve_count"))
	for child in node.get_children():
		total += _linkage_solve_count(child)
	return total

func _disable_visual_suspension(node: Node) -> int:
	var disabled := 0
	for child in node.get_children():
		if child.get("enable_suspension_geometry") != null:
			child.set("enable_suspension_geometry", false)
			disabled += 1
		disabled += _disable_visual_suspension(child)
	return disabled

## SUS-GEO-12 attribution: temporarily disable processing on each process
## active node and measure a short window, ranking the frame-time consumers.
func _bisect_process_consumers(compositor: Node, vehicle: Node, window_frames: int) -> void:
	var candidates: Array = []
	_collect_process_nodes(compositor, candidates)
	print("[BISECT] process-active nodes: %d" % candidates.size())
	for candidate in candidates:
		if not is_instance_valid(candidate) or not candidate.is_processing():
			continue
		var label := "%s (%s)" % [candidate.get_path(), candidate.get_class()]
		candidate.set_process(false)
		var samples: Array = []
		for _frame in window_frames:
			var before_usec := Time.get_ticks_usec()
			await process_frame
			samples.append(float(Time.get_ticks_usec() - before_usec) / 1000.0)
		candidate.set_process(true)
		samples.sort()
		print("[BISECT] %s -> frame_ms p50=%.3f p95=%.3f" % [label, samples[window_frames / 2], samples[int(window_frames * 0.95)]])

## SUS-GEO-12 attribution: with every process-active node disabled, enable one
## at a time and measure a short window (true per-node cost, no masking).
func _buildup_process_consumers(compositor: Node, window_frames: int) -> void:
	var candidates: Array = []
	_collect_process_nodes(compositor, candidates)
	print("[BUILDUP] process-active nodes: %d" % candidates.size())
	for candidate in candidates:
		if is_instance_valid(candidate):
			candidate.set_process(false)
	var samples: Array = []
	for _frame in window_frames:
		var before_usec := Time.get_ticks_usec()
		await process_frame
		samples.append(float(Time.get_ticks_usec() - before_usec) / 1000.0)
	samples.sort()
	print("[BUILDUP] all_off -> frame_ms p50=%.3f p95=%.3f" % [samples[window_frames / 2], samples[int(window_frames * 0.95)]])
	for candidate in candidates:
		if not is_instance_valid(candidate):
			continue
		candidate.set_process(true)
		for _frame in 10:
			await process_frame
		var window: Array = []
		for _frame in window_frames:
			var before_usec := Time.get_ticks_usec()
			await process_frame
			window.append(float(Time.get_ticks_usec() - before_usec) / 1000.0)
		candidate.set_process(false)
		window.sort()
		print("[BUILDUP] %s (%s) -> frame_ms p50=%.3f p95=%.3f" % [
			candidate.get_path(), candidate.get_class(), window[window_frames / 2], window[int(window_frames * 0.95)]])
	for candidate in candidates:
		if is_instance_valid(candidate):
			candidate.set_process(true)

func _collect_process_nodes(node: Node, out: Array) -> void:
	for child in node.get_children():
		if child.is_processing():
			out.append(child)
		_collect_process_nodes(child, out)

## SUS-GEO-12 attribution: alternate F90Core processing on/off in blocks and
## read Performance.TIME_PROCESS per block (direct _process cost evidence).
func _ab_block(node_path: String, window_frames: int) -> void:
	var core := root.get_node_or_null(node_path)
	if core == null:
		printerr("[AB] node not found: " + node_path)
		return
	var on_ms: Array = []
	var off_ms: Array = []
	for block in range(6):
		var target_on := block % 2 == 0
		core.set_process(target_on)
		for _frame in 15:
			await process_frame
		var samples: Array = []
		for _frame in window_frames:
			await process_frame
			samples.append(Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0)
		samples.sort()
		var p50: float = samples[samples.size() / 2]
		if target_on:
			on_ms.append(p50)
		else:
			off_ms.append(p50)
	core.set_process(true)
	print("[AB] %s ON p50s=%s OFF p50s=%s" % [node_path, str(on_ms), str(off_ms)])

## SUS-GEO-12 attribution: with every other process node disabled, alternate the
## target node on/off and measure the raw frame delta per block.
func _ab_isolated(target: Node, window_frames: int) -> void:
	var others: Array = []
	_collect_process_nodes(root, others)
	for node in others:
		if is_instance_valid(node) and node != target:
			node.set_process(false)
	var on_ms: Array = []
	var off_ms: Array = []
	for block in range(6):
		var target_on := block % 2 == 0
		target.set_process(target_on)
		for _frame in 15:
			await process_frame
		var samples: Array = []
		for _frame in window_frames:
			var before_usec := Time.get_ticks_usec()
			await process_frame
			samples.append(float(Time.get_ticks_usec() - before_usec) / 1000.0)
		samples.sort()
		var p50: float = samples[samples.size() / 2]
		if target_on:
			on_ms.append(p50)
		else:
			off_ms.append(p50)
	target.set_process(true)
	for node in others:
		if is_instance_valid(node):
			node.set_process(true)
	print("[AB] isolated %s ON p50s=%s OFF p50s=%s" % [str(target.get_path()), str(on_ms), str(off_ms)])

func _percentiles(samples: Array) -> Dictionary:
	var sorted_values := samples.duplicate()
	sorted_values.sort()
	var n := sorted_values.size()
	return {
		"p50": sorted_values[int(n * 0.5)],
		"p95": sorted_values[int(n * 0.95)],
		"p99": sorted_values[int(n * 0.99)],
		"max": sorted_values[n - 1],
		"avg": _avg(samples),
	}

func _avg(samples: Array) -> float:
	var total := 0.0
	for value in samples:
		total += value
	return total / maxf(float(samples.size()), 1.0)

func _fmt(stats: Dictionary) -> String:
	return "avg=%.3f p50=%.3f p95=%.3f p99=%.3f max=%.3f" % [
		stats["avg"], stats["p50"], stats["p95"], stats["p99"], stats["max"]]

func _run() -> void:
	var scene_path := _argument("--scene", DEFAULT_SCENE_PATH)
	var packed := load(scene_path) as PackedScene
	if packed == null:
		printerr("[CAPTURE][FAIL] Could not load %s" % scene_path)
		quit(1)
		return
	var compositor := packed.instantiate()
	root.add_child(compositor)
	for _frame in 8:
		await process_frame
	var vehicle := compositor.find_child("VehicleRigidBody", true, false)
	if vehicle == null:
		printerr("[CAPTURE][FAIL] Could not find VehicleRigidBody")
		quit(1)
		return
	vehicle.set("enable_player_input", false)

	var no_visual := _flag("--no-visual-suspension")
	var solves_before := 0
	if no_visual:
		var disabled := _disable_visual_suspension(compositor)
		print("[CAPTURE] visual suspension controllers disabled: %d" % disabled)
		solves_before = _linkage_solve_count(compositor)

	# Audio attribution: F90Core owns the audio pump in _process; disabling
	# before add_child skips the generator/player creation entirely.
	var no_audio := _flag("--no-audio")
	if no_audio:
		var core := compositor.get_node_or_null("F90Core")
		if core != null:
			core.set("enable_audio", false)
			print("[CAPTURE] F90Core enable_audio=false (before _ready)")
		else:
			printerr("[CAPTURE][FAIL] F90Core node not found for audio attribution")

	# VSync off: frame deltas must expose the render cost, not the refresh clamp.
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)

	var throttle := float(_argument("--throttle", "0.0"))
	var warmup := int(_argument("--warmup", str(WARMUP_FRAMES)))
	var measure_frames := int(_argument("--frames", str(MEASURE_FRAMES)))

	for _frame in warmup:
		await process_frame

	if _flag("--bisect"):
		await _bisect_process_consumers(compositor, vehicle, 120)
	if _flag("--buildup"):
		await _buildup_process_consumers(compositor, 150)
	if _flag("--ab-f90core"):
		await _ab_block("/root/VehicleTestSession/F90Core", 120)
	if _flag("--ab-isolated"):
		var core_node := root.get_node_or_null(NodePath("VehicleTestSession/F90Core"))
		if core_node != null:
			await _ab_isolated(core_node, 120)
		else:
			printerr("[CAPTURE][FAIL] F90Core node not found")

	var frame_ms: Array = []
	var physics_ms: Array = []
	var process_ms: Array = []
	# Per-second FPS buckets expose sustained drift (the aggregate percentiles
	# above hide a monotonic degradation over the window).
	var fps_buckets: Array = []
	var bucket_start_usec := Time.get_ticks_usec()
	var bucket_frames := 0
	for _frame in measure_frames:
		if throttle > 0.0 and vehicle.get("throttle_amount") != null:
			vehicle.set("throttle_amount", throttle)
		var before_usec := Time.get_ticks_usec()
		await process_frame
		frame_ms.append(float(Time.get_ticks_usec() - before_usec) / 1000.0)
		physics_ms.append(Performance.get_monitor(Performance.TIME_PHYSICS_PROCESS) * 1000.0)
		process_ms.append(Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0)
		bucket_frames += 1
		var bucket_usec := Time.get_ticks_usec() - bucket_start_usec
		if bucket_usec >= 1_000_000:
			fps_buckets.append(float(bucket_frames) * 1_000_000.0 / float(bucket_usec))
			bucket_start_usec = Time.get_ticks_usec()
			bucket_frames = 0
	if bucket_frames > 0:
		var tail_usec := Time.get_ticks_usec() - bucket_start_usec
		if tail_usec > 0:
			fps_buckets.append(float(bucket_frames) * 1_000_000.0 / float(tail_usec))

	var frame := _percentiles(frame_ms)
	var physics := _percentiles(physics_ms)
	var process := _percentiles(process_ms)
	var fps := Engine.get_frames_per_second()

	print("[CAPTURE] frame_ms %s | fps=%.1f" % [_fmt(frame), fps])
	print("[CAPTURE] physics_monitor_ms %s" % _fmt(physics))
	print("[CAPTURE] process_monitor_ms %s" % _fmt(process))
	var bucket_labels: Array = []
	for bucket_index in fps_buckets.size():
		bucket_labels.append("%ds=%.1f" % [bucket_index + 1, fps_buckets[bucket_index]])
	print("[CAPTURE] fps_buckets %s" % " ".join(bucket_labels))
	print("[CAPTURE] draw_calls=%d objects=%d primitives=%d" % [
		RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME),
		RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_OBJECTS_IN_FRAME),
		RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_PRIMITIVES_IN_FRAME),
	])
	var solves_after := _linkage_solve_count(compositor)
	print("[CAPTURE] linkage_solves before=%d after=%d (no_visual=%s)" % [solves_before, solves_after, str(no_visual)])
	if no_visual and solves_after != solves_before:
		printerr("[CAPTURE][FAIL] linkage solves continued while disabled")

	var out_path := _argument("--out", "")
	if not out_path.is_empty():
		var payload := {
			"scene": scene_path,
			"throttle": throttle,
			"frames": measure_frames,
			"no_visual": no_visual,
			"frame_ms": frame,
			"physics_monitor_ms": physics,
			"process_monitor_ms": process,
			"fps": fps,
			"fps_buckets": fps_buckets,
			"draw_calls": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME),
			"objects": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_OBJECTS_IN_FRAME),
			"primitives": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_PRIMITIVES_IN_FRAME),
		}
		var file := FileAccess.open(out_path, FileAccess.WRITE)
		if file != null:
			file.store_string(JSON.stringify(payload))
			print("[CAPTURE] wrote %s" % out_path)

	compositor.queue_free()
	quit(0)
