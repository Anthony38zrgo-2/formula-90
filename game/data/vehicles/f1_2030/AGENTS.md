# f1_2030 — 2030 vehicle data

Scope: physics, geometric, and suspension-mesh data for the 2030 V10 car.
The `underfloor_contact` profile section owns the approximation, activation and
release clearances, progressive contact rates, rigid-contact eligibility and
hold/filter times. Rust validates and evaluates these values; the native bridge
only submits raw probes and rigid contacts. The current clearances are 35 mm,
25 mm and 50 mm respectively. Loaded individual probes can activate scrape audio.
Consumers: the physics engine, suspension visuals, and 2030 vehicle scenes.
Rules: geometry and physics stay consistent by construction; a mesh change revalidates the packaging tests.

The geometric profile carries the component mass inventory and physical suspension hardpoints. The preceding eighteen-inch package had a 600 kilogram vehicle-and-driver budget plus 98 kilograms of rims and tires. The thirteen-inch package recalculates the base to 597.7950900124142 kilograms and rims and tires to 80.54280625740013 kilograms, giving 678.3378962698143 kilograms dry before fuel. Component masses use uniform thin-shell surface integration calibrated to the preceding estimates, with explicit uncertainty and no ballast compensation. Tire diameter is 670 millimeters, widths remain 330 and 420 millimeters, and brake discs measure 280 millimeters on both axles. Brake thermal rotor masses follow the component inventory; rim heat capacities follow the revised rim masses. Maximum brake torque scales with disc diameter under a constant clamp-force assumption. Spring free lengths are recalibrated from the actual rest-pose motion ratios and supported loads. Treat component allocations as estimates with uncertainty; update dependent audits when changing them.

Validate rear chassis clearance throughout suspension travel, damper anchorage, kinematic convergence and force transmission. Keep physical hardpoints separate from cosmetic visual packaging. Profile edits must regenerate the profile digests in both original and MP4/6 livery manifests through the existing validation workflow.

The `profiles/geometric` root is the versioned manifest source for the geometric profile; the flat geometric JSON stays frozen as temporary compatibility until every consumer migrates. Edit the domain documents there, keep every value identical to the frozen monolith until the migration completes, and never let the two diverge silently.
