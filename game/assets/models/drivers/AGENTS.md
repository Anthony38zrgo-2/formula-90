# drivers — Seated driver characters

Scope: driver.glb and its source provenance manifest, derived from Racer.fbx.
Consumers: the original F1 2030 vehicle scene and driver visual controller.
Rules: regenerate through tools/drivers/generate_driver_model.py. Preserve the pit crew program. Runtime hands follow steering targets through skeletal modifiers. The independent driver rig has fifteen articulated bones per glove, including an opposable thumb; fingers open only for the hand changing grip.
