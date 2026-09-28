# vehicles — Vehicle model programs

Scope: one program folder per vehicle generation, each with sources, runtime meshes, and livery textures.
Consumers: vehicle scenes under `game/scenes/vehicles/` and `tools/blender/`.
Rules: cross-vehicle sharing goes through `game/assets/materials/` and `game/assets/textures/`; never copy a mesh between programs.
Subfolders: each child folder documents itself in its own AGENTS.md.
