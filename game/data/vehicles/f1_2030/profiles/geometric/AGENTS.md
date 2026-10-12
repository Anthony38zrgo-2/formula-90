# geometric — F1 2030 geometric profile manifest

Scope: the versioned domain documents split from f1_2030_v10_geometric.json with every value preserved.
Consumers: physics manifest loading, the build.rs embedded resolved document, and formula90-core complete-profile composition.
Rules: this root is the editable source for the geometric profile; the monolith f1_2030_v10_geometric.json stays frozen as temporary compatibility and must not diverge. suspension_presentation.json keeps the visual suspension geometry (including `_visual_meshes`) for presentation consumers; audio.json is owned by vehicle-audio-engine and collected by formula90-core.
