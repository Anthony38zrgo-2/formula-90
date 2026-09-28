# tools — Authoring and validation pipeline

Scope: the Python and C++ pipeline building, validating, and diagnosing game content: audio banks, Blender art, physics telemetry, validators, and shared infrastructure. `build_native.py` drives native builds from here.
Consumers: developers, `scripts/` entry points, and CI gates.
Rules: domain-neutral infrastructure lives in `common/` and never absorbs domain rules. All file writes go through `common/output_policy.py`: previews to `scratch/`, promotions to `game/assets/` or `game/sounds/`.
Subfolders: each child folder documents itself in its own AGENTS.md.
