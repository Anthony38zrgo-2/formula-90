# drivers — Driver character authoring

Scope: reproducible seated driver models derived from the supplied low polygon driver Blender source, plus the legacy Racer generator.
Consumers: vehicle driver visuals under game/scripts/vehicle.
Rules: validate preview and promotion destinations through tools/common/output_policy.py; preserve the pit crew source and outputs. The calibrated torso recline stays fixed when the vehicle seat is reshaped around the driver.

generate_driver_model.py defaults to the low_polygon character model. Its source package contains the original Blender model, six PNG textures and a neutral reference skeleton. The source skeleton preserves the existing arm and camera contract. low_polygon_driver_model.py adapts geometry and deformation weights to that skeleton and authors a fixed curved glove grip; hand motion follows the wrist instead of deforming individual fingers. The original character model remains an explicit option. Do not overwrite the reference skeleton during regeneration. Preview destinations contain the exported model, manifest and editable prepared Blender source. Promotion writes the prepared Blender source inside the ignored source folder.
