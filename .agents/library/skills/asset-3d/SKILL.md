# 3D Asset

SCOPE: mesh inspection/mutation; UV/material prep; simplification; component naming; Godot-ready export.

FLOW:
inspect -> geometry/component analysis -> smallest delta -> deterministic tooling -> Blender? -> export-validate -> import-validate.

POLICY:
DETERMINISTIC-FIRST; PY/MESH-TOOLS>PREFER; BLENDER=LAST-STAGE; LOAD(vehicle-physics) only when geometry/contract is physics-relevant.
