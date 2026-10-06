# f1_2030 — 2030 vehicle data

Scope: physics, geometric, and suspension-mesh data for the 2030 V10 car.
Consumers: the physics engine, suspension visuals, and 2030 vehicle scenes.
Rules: geometry and physics stay consistent by construction; a mesh change revalidates the packaging tests.

The geometric profile carries the component mass inventory and physical suspension hardpoints. The 600 kg base budget includes the vehicle and driver, excludes rims/tires and excludes fuel. The current rim/tire budget totals 98 kg, giving 698 kg dry operating mass before fuel. Treat component allocations as estimates with uncertainty; update dependent audits when changing them.

Validate rear chassis clearance throughout suspension travel, damper anchorage, kinematic convergence and force transmission. Keep physical hardpoints separate from cosmetic visual packaging. Profile edits must regenerate the profile digests in both original and MP4/6 livery manifests through the existing validation workflow.
