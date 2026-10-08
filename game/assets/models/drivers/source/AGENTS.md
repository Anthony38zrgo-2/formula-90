# Driver authoring sources

Scope: the supplied Blender model, textures, neutral reference skeleton, source provenance and prepared seated Blender output.
Consumers: tools/drivers/generate_driver_model.py and low_polygon_driver_model.py.
Rules: this folder is excluded from Godot import by .gdignore. Preserve original supplied model and texture hashes. Regenerate prepared_driver.blend and runtime exports through the driver generator. The source/ and textures/ children are binary leaves covered here. Promotion requires tools/common/output_policy.py validation.
