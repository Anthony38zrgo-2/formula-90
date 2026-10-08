# vehicle — Vehicle GDScript

Scope: driving aids, wheel and suspension visuals, tuning contracts, and the Rust controller bridge.
Consumers: vehicle scenes and suspension test suites.
Rules: visual pose mirrors physics state; aids tuning stays reviewable in data and tests.

Under physical-world ownership, wheel and suspension renderers consume the Rust corner solutions, hub bases, spin and linkage endpoints. Derive rigid visual transforms only. Bypass the second linkage solve and cosmetic front packaging in this mode. Preserve legacy rendering for sessions that do not select the candidate. Validate hub and damper agreement and rear chassis clearance across travel; visual agreement does not certify physical forces.

share_formula_one_2030_tire_material_on_import.gd is an EditorScenePostImport hook used only by the four canonical F1 2030 wheel GLBs. It replaces the tire ArrayMesh surface material with the external shared tire resource while preserving the rest of the imported scene. Exactly one tire surface is required per export. Replacing the material on the mesh, rather than adding an instance override, prevents the embedded tire textures from remaining resident in the cached runtime scenes. The hook performs no wheel pose or physics changes.
