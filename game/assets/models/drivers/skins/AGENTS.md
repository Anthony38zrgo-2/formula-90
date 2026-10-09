# Driver appearance sources

Scope: editable appearance profiles and texture atlases for the current low polygon driver.
Consumers: tools/drivers/apply_lewis_hamilton_driver_skin.py and generate_driver_model.py.
Rules: this folder is excluded from Godot import. Runtime textures are embedded in driver.glb. Each appearance child is a binary leaf with its appearance profile and generation specification covered by this file. Preserve original supplied textures under source/textures. Appearance changes must preserve mesh geometry, deformation weights, skeleton, seating and DriverEyePoint. Never apply these profiles to the deprecated Racer character.

lewis_hamilton_inspired uses a yellow helmet with red contours and stars, red and white suit, red gloves, recognizable 44 markings and a warm reflective visor. Its helmet atlas includes vertical margins; the profile records the original aspect ratio for normalized texture coordinate alignment. Generate the original prepared low polygon driver first, then apply the appearance exactly once.
