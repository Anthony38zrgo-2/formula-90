# f1-2030 — 2030 vehicle program

Scope: the 2030 V10 meshes, named livery sets, Blender sources, and export validation outputs.
Consumers: the `f1_2030_v10` scenes and `tools/blender/` remodel and review scripts.
Rules: one folder per named livery under `liveries/`; shared source textures stay in `source/textures/`.
Subfolders: each child folder documents itself in its own AGENTS.md.

The program manifest pins the geometric vehicle profile digest. Mass or physical suspension profile changes require a matching manifest update and launcher validation. Mesh clearance evidence must cover the physical rear linkage and damper travel; a static render alone cannot validate the full travel envelope.

formula_one_2030_tire_material.tres shares FormulaOne2030TireColor.png and FormulaOne2030TireParameters.png across all four wheel exports. The four GLB import descriptors invoke game/scripts/vehicle/share_formula_one_2030_tire_material_on_import.gd, replacing each tire surface material during import. This avoids four resident copies of the same tire textures. The shared runtime PNGs are promoted from source/textures/reference_balanced_finish/ and must match those source maps. Preserve these resource paths and import settings when regenerating wheel exports. source/reference_finish_report.json records the current finish, authoring-to-runtime wheel surfaces, triangle counts and Compatibility renderer verification. The reference finish uses the existing sky reflections and material roughness; it introduces no additional runtime reflection probes or global environment changes.

The two shared runtime tire texture import descriptors enable mipmaps and high quality GPU compression. The shared material uses linear filtering with mipmaps and anisotropy. Preserve those settings for legible lettering, stable distant sidewalls and the measured texture memory budget.
