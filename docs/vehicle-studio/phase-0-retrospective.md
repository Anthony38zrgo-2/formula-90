# Vehicle Studio Phase 0 Retrospective

Status: `DONE`

Human gate: `ACCEPTED`

## Result

Phase 0 established an immutable Williams fingerprint, separated current facts
from historical report claims, froze VehicleDocument and VehicleView contracts,
and produced a complete primitive/frame mapping without mutating the source.

## What worked

- Reusing the existing GLB inspector produced deterministic structural evidence.
- Content hashing made the untracked imported baseline safe to reference.
- Modular authoring sources removed duplicate full-chassis ownership.
- Schema fixtures exposed the boundary between structural and semantic
  validation before domain code existed.
- Negative SVG fixtures fixed the security boundary early.
- The mapping coverage check prevented silent primitive omission.

## What remains intentionally deferred

- Persistent vertex bindings for embedded sidepod, engine-cover and protected
  chassis regions require the onboarding UI.
- Wing element-level bindings are not yet confirmed.
- Historical refinement reproducibility remains unproven.
- Normal completion is an export/materializer responsibility.
- The unavailable agentdb seed mechanism remains a tooling backlog issue.

## Decision carried forward

Revision zero may be compiled only after interactive onboarding records the
remaining region confirmations. Until then the accepted mapping is an import
contract, not a completed editable VehicleDocument.

## Next item

`VS-010`: pure-Python VehicleDocument domain, diagnostics and canonical
serialization.
