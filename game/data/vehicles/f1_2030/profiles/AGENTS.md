# profiles — Versioned vehicle profile roots

Scope: one manifest root per vehicle profile; each root holds vehicle.json plus its domain documents.
Consumers: the physics loader, formula90-core profile coordination, and offline diagnostics.
Rules: manifest references are relative to the manifest and must stay under the authorized data root; unknown domains, unknown manifest fields and unknown document fields are rejected; audio and suspension presentation belong to their own consumers, not to the physics loader.
