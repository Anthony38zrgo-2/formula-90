# drivers — Seated driver characters

Scope: driver.glb and its source provenance manifest, derived from the supplied low polygon race car driver Blender model.
Consumers: the original F1 2030 vehicle scene and driver visual controller.
Rules: regenerate through tools/drivers/generate_driver_model.py. Preserve the pit crew program. Runtime hands follow steering targets through skeletal modifiers. The low polygon model uses authored curved gloves attached to the wrist; finger geometry does not deform. The existing fifty-five bone contract remains compatible with the arm and camera controllers. Head and neck geometry is separate from the body. DriverEyePoint is attached to the head bone and calibrated from the helmet visor; cockpit cameras follow that authored marker.

source/ contains the original Blender model, PNG textures, neutral reference skeleton and prepared seated Blender model. Its .gdignore excludes authoring resources from Godot import. Source and texture hashes are recorded in driver_manifest.json. Preview outputs and backups live under scratch/driver_model_replacement_20261008. Source promotion requires output_policy.py validation.
