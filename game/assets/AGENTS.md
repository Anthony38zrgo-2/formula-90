# assets — Binary art and authored sources

Scope: binary art and Blender-authored sources consumed through `res://` paths: models, sprites, materials, themes, and track textures. The `fonts/` folder holds only the bundled Barlow typeface reference; the runtime typeface home is `game/fonts/`.
Consumers: Godot scenes, the Blender pipeline in `tools/blender/`, and the pit-crew generator in `tools/pit_crew/`.
Rules: promotion-only writes, enforced by `tools/common/output_policy.py`: previews go to `scratch/`, approved content is promoted here. Pure binary leaves carry no AGENTS.md and are covered by their parent.
Subfolders: each child folder documents itself in its own AGENTS.md.
