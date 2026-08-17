# Godot Runtime

SCOPE: scene/runtime; GDExtension; node lifecycle; startup-blocking imports; Godot<->Rust integration; runtime errors.

FLOW:
DIAG-FIRST -> classify failure-stage{pre-start|extension-load|scene-instantiation|gameplay-init|first-valid-physics-frame} -> smallest responsible delta -> REAL-RUNTIME -> persist logs/exit -> validate readiness.

POLICY:
NO-SMOKE; `[OBSERVABILITY] GAME_READY` valid only after project-defined readiness; missing observation => `NOT_OBSERVED`; LOAD(asset-3d) only for asset/import causality.
