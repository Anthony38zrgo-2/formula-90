# vehicle — Vehicle GDScript

Scope: driving aids, wheel and suspension visuals, tuning contracts, and the Rust controller bridge.
Consumers: vehicle scenes and suspension test suites.
Rules: visual pose mirrors physics state; aids tuning stays reviewable in data and tests.

Under physical-world ownership, wheel and suspension renderers consume the Rust corner solutions, hub bases, spin and linkage endpoints. Derive rigid visual transforms only. Bypass the second linkage solve and cosmetic front packaging in this mode. Preserve legacy rendering for sessions that do not select the candidate. Validate hub and damper agreement and rear chassis clearance across travel; visual agreement does not certify physical forces.
