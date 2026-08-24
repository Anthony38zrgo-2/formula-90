# VS-022 — Deterministic orthographic SVG compiler

Status: READY_FOR_HUMAN_GATE

## Outcome

VehicleDocument revision zero now compiles to top, side, front and rear SVG
views plus a semantic JSON manifest. Geometry access reuses the established
GLB reader, accessor and transform functions.

Each SVG contains fixed schema and view markers, persistent component IDs,
document/source hashes, deterministic projected edges and a geometry-derived
viewBox.

## Williams proof

- top: 8175 edges; SHA-256 C029EFCF16BC9FEB96A6833C22A8BC815255CD5D2253FC2F8D5AB2A4B8632057
- side: 5059 edges; SHA-256 9C803E28B72F9200C13C4395B8468C6420A1C76B792594D6BFB1439346DC518E
- front: 10277 edges; SHA-256 4D3CADA8B9163EAC623CBDA2FB7EDA97F57C6E90EEBA3FB221B23DA8D659FB8E
- rear: 10277 edges; SHA-256 7590567D1F73F2CCAE541D63BB0270770C8D3AD9E01EA8D4104B02AEDF26CF5C

The files are currently generated and displayed in memory. Persistent project
storage and export belong to later backlog items.

## Verification

- 51 Python tests pass.
- 2 Vue tests pass.
- TypeScript and Vite production build pass.
- Repeated fixture compilations produce byte-identical SVG and manifest.
- Williams source assets remain read-only.
