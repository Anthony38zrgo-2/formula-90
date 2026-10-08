# textures — 2030 source texture packs

Scope: source texture packs shared by the 2030 liveries.
Consumers: the livery baking scripts in `tools/blender/`.
Rules: one pack per named livery; packs here are inputs, the baked runtime set lives beside the meshes.

chassis_flat_decals/ is a binary leaf containing two generated 4096 by 4096 base color PNGs for the canonical chassis red lacquer and satin carbon materials. It contains no scripts, configuration or scenes. tools/blender/convert_formula_one_2030_chassis_decals_to_textures.py projects the historical sponsor and flag meshes into existing chassis texture coordinates and composites their colors into these maps. No decal geometry or relief is added. Regenerate and validate under scratch/ before promotion; preserve the original texture packs and non-color material maps.

reference_balanced_finish/ is a binary leaf containing the generated PNGs for the subsequent reference finish rebalance: clean authoring colors, chassis colors with flat sponsor artwork, packed material parameters, and shared tire color and parameters. tools/blender/rebalance_formula_one_2030_reference_finishes.py regenerates these maps under scratch/. The original source atlas and earlier packs remain available as inputs. Tire lettering and the white band affect color only. The runtime tire color and parameters are copied beside the five GLBs and referenced by the shared runtime material; promotion must keep those two copies pixel identical to their source maps. Godot excludes the source tree through .gdignore, so runtime resources must reference the copies beside the meshes.
