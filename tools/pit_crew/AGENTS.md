# pit_crew — Pit-crew pose generation

Scope: the generators producing pit-crew pose meshes from source models: `generate_animated_pit_crew_rigs.py` (rigged + animated clips, current runtime target) and `generate_static_pit_crew_poses.py` (frozen static poses, retained for provenance).
Consumers: `game/assets/models/pit_crew/racer/` and its pose manifest.
Rules: generated poses are reproducible from sources; the manifest stays in sync with every generation. The rigged generator is the one whose output matches the tracked poses.
