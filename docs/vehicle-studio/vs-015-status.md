# VS-015 — Initial revision compiler

Status: READY_FOR_HUMAN_GATE

## Outcome

Accepted onboarding state now compiles through a pure function into a valid
VehicleDocument revision zero. Confirmation command count, confirmation order,
timestamps and temporary absolute paths do not enter canonical bytes.

The Williams end-to-end in-memory proof produced:

- revision ID: revision-zero-5b934fae1020
- document SHA-256: FD3279997D0876E90B0F87065DEED82029E06B672D55B5CC690E7F6FC43629DB
- components: 12
- frames: 9
- diagnostics: 0

## Contract decisions

- Source authority is the aggregate hash of the modular GLB bundle.
- Components retain their source file and source object identity.
- Wheel frames retain the accepted anchor-map translations.
- Axle centers are derived from paired wheel anchors.
- Wing mounts come from explicit GLB nodes using the existing parser.
- Topology changes and source mutation remain prohibited.
- The compiler only returns canonical state; it does not write or build assets.

## Verification

- Equivalent mappings with different confirmation order produce identical bytes.
- Incomplete mappings are rejected.
- The compiled Williams document passes all domain diagnostics.
- Vue exposes Create revision initial only after the completeness gate passes.
