# reference_red_and_gold — Reference paint and sponsor pack

Scope: generated red chassis lacquer, champagne gold wheel finish, and transparent SF-26 sponsor decal atlas.
Consumers: the canonical f1_2030.blend source.
Rules: preserve generated source images and original vehicle geometry. Blender materials provide reflections and roughness. sponsor_decal_layout.json records historical placements adapted from the lower 2026 car in the supplied reference. The canonical chassis and tires now use flat color textures under ../reference_balanced_finish/. Historical chassis and tire decal meshes are projection inputs retained only in the backups recorded by ../../chassis_flat_decal_report.json and ../../reference_finish_report.json. The tire printing follows wheel rotation through the tire mesh texture coordinates and adds no geometry. Runtime exports are regenerated separately after visual review.
