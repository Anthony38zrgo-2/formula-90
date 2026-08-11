extends SceneTree


func _init() -> void:
	var failures := 0
	var track_scene := load("res://scenes/tracks/test_field/la_chutana_generated.tscn") as PackedScene
	if track_scene == null:
		printerr("[FAIL] La Chutana generated scene could not load.")
		quit(1)
		return

	var track := track_scene.instantiate()
	var legacy_overlay := track.get_node_or_null("Skybox2D")
	if legacy_overlay != null:
		printerr("[FAIL] The CanvasLayer skybox overlay must not remain in the 3D track scene.")
		failures += 1

	var source_rig := track.get_node_or_null("SourceSkyboxRig") as Node3D
	if source_rig == null:
		printerr("[FAIL] SourceSkyboxRig instance is missing.")
		failures += 1
	else:
		var mountain_cards: Array[Sprite3D] = []
		var waterfalls: Array[Sprite3D] = []
		for descendant in source_rig.find_children("*", "", true, false):
			if descendant is CollisionObject3D or descendant is NavigationRegion3D:
				printerr("[FAIL] SourceSkyboxRig contains non-visual node: %s" % descendant.get_path())
				failures += 1
			if descendant is Sprite3D:
				var sprite := descendant as Sprite3D
				if sprite.is_in_group(&"source_skybox_waterfalls"):
					waterfalls.append(sprite)
				else:
					mountain_cards.append(sprite)

		if mountain_cards.size() != 8:
			printerr("[FAIL] Expected 8 distant 3D mountain cards, found %d." % mountain_cards.size())
			failures += 1
		for mountain in mountain_cards:
			if mountain.texture == null or mountain.no_depth_test or mountain.shaded or mountain.cast_shadow != GeometryInstance3D.SHADOW_CASTING_SETTING_OFF:
				printerr("[FAIL] Mountain card %s violates visual/depth configuration." % mountain.get_path())
				failures += 1

		if waterfalls.size() != 16:
			printerr("[FAIL] Expected 16 animated waterfall sprites, found %d." % waterfalls.size())
			failures += 1
		for waterfall in waterfalls:
			if waterfall.texture == null or waterfall.hframes != 4 or waterfall.no_depth_test or waterfall.cast_shadow != GeometryInstance3D.SHADOW_CASTING_SETTING_OFF:
				printerr("[FAIL] Waterfall %s violates atlas/depth configuration." % waterfall.get_path())
				failures += 1

	var world_environment := track.get_node_or_null("WorldEnvironment") as WorldEnvironment
	if world_environment == null or world_environment.environment == null:
		printerr("[FAIL] WorldEnvironment is missing from La Chutana.")
		failures += 1
	else:
		var sky := world_environment.environment.sky
		var panorama := sky.sky_material as PanoramaSkyMaterial if sky else null
		if panorama == null or panorama.panorama == null or panorama.filter:
			printerr("[FAIL] La Chutana must use an unfiltered panorama sky material.")
			failures += 1

	track.queue_free()
	if failures == 0:
		print("[PASS] La Chutana source-style skybox is visual-only, depth-tested, and uses the panorama/waterfall atlas.")
	quit(failures)
