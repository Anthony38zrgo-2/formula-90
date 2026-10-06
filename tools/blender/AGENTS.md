# blender — Blender authoring scripts

Scope: Blender scripts remodeling, texturing, exporting, rendering, and validating the F1 vehicle meshes and liveries.
Consumers: the vehicle programs under `game/assets/models/vehicles/`.
Rules: scripts transform sources into regenerable outputs; review renders are evidence beside the sources they validate.

`validate_rear_suspension_chassis_clearance.py` audits physical rear suspension packaging against chassis geometry. Use the current physical profile and source geometry, evaluate the travel envelope and save evidence under `scratch/`. Include damper anchorage and moving linkage clearance. Keep cosmetic pose adjustments separate from physical geometry validation.
