# VS-024 — Before/after interactive 3D preview

Status: READY_FOR_HUMAN_GATE

## Outcome

Vehicle Studio now offers a lazy-loaded Three.js preview next to the linked
CAD panels. Before and after use the same GLB, camera recipe, target, lighting
and synchronized orbit controls. Until deformation commands exist, the two
models are intentionally identical.

Both panels explicitly identify browser rendering as non-authoritative; fixed
Blender preview renders remain a separate later backlog item.

## Security boundary

- The client requests an allowlisted asset ID, never a filesystem path.
- The API binds asset IDs to one repository-confined GLB.
- Unknown IDs return a structured rejection.
- The source file is streamed read-only as model/gltf-binary.
- Three.js is lazy-loaded, reducing the initial editor chunk to about 80 kB.

## Onboarding improvement

The semantic confirmation screen now includes Confirmar todas las piezas.
It chooses the first stable suggestion per unmapped role, preserves individual
evidence, rejects duplicate-role batches and commits the batch in one revision.

## Verification

- 54 Python tests pass.
- 5 Vue tests pass.
- TypeScript/Vite production build passes.
- Loopback integration returned the expected camera recipe.
- The GLB response was model/gltf-binary with 604164 bytes.
- npm audit reported zero vulnerabilities.
