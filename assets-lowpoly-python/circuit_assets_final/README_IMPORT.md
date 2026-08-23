# Circuit asset import contract

- Runtime geometry is GLB under `assets/<id>/<id>.glb`.
- Every GLB has `assets/<id>/<id>.json` with exact bounds and dimensions measured from the exported GLB.
- Units are interpreted as meters, scale is baked to `1,1,1`, Y is up, and no axis rotation is applied.
- Every asset is rebased to a bottom-center origin: local `Y=0` is the lowest geometry point.
- `normalization.source_bottom_center_m` records the original AC3D anchor. Translating the normalized instance by that value reconstructs the original source placement.
- Textures referenced by each AC3D object are embedded in its GLB. PNG copies are kept beside the asset for inspection/editing.
- The complete convertible source texture bank is also normalized to PNG under `texture_library/`.
- Collision is not authored by AC3D source metadata; JSON contains only a recommendation and does not silently invent collision geometry.
- For deterministic placement, do not auto-rescale GLBs in Godot. Use scale `1.0` and read dimensions/AABB from JSON.
