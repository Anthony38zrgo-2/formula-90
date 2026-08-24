# Vehicle Studio — Sprint 1 progress

Status: READY_FOR_HUMAN_GATE for VS-010 through VS-013, VS-020 and VS-021.
VS-014 remains IN_PROGRESS.

Provenance at review:

- branch: f1-94
- HEAD: 506386ca5d08bd646d38437ce6e1b1ee3c96a922
- Williams source directory remains untracked and was never written by the
  implementation or test commands.
- Phase 0 accepted source fingerprint:
  3C7B996CFAB2EDF263E223A06846BF84C6822CD343D21E03561CD9AD19FB90A9

## Reviewed items

### VS-010 — VehicleDocument domain library

- Strict semantic validation with stable diagnostics.
- Canonical JSON uses explicit ordering and fixed finite-number rules.
- Required vehicle frames, references, axes and no-topology policy are checked.

### VS-011 — GLB scan adapter

- Reuses the existing GLB inspector rather than duplicating parsing.
- Confines input to the repository and preserves the source hash.
- Produces byte-stable normalized evidence.

### VS-012 — Blend read-only adapter

- Runs Blender with factory startup and a purpose-built read-only probe.
- Reports evaluated mesh bounds/counts, UVs, materials, modifiers, transforms,
  collections and custom properties.
- Integration tests create a disposable fixture and verify identical scans and
  unchanged source bytes.

### VS-013 — Semantic suggestion engine

- Rules are generic for F1 open-wheel naming, with no model-name branch.
- Suggestions include evidence, stable IDs and suggestion_only authority.
- Unknown names are not guessed; every result requires human confirmation.

### VS-020 — Local project service

- Project identifiers and source paths are confined.
- Only typed, allowlisted commands are accepted.
- Optimistic revision checks reject concurrent stale commands.
- Save/state mutation never schedules a build; build request is explicit.

### VS-021 — Vue/Tailwind application shell

- Isolated Vue 3, Tailwind 4 and Vite application with reproducible lockfile.
- Loads a canonical project snapshot in read-only presentation mode.
- Includes project inspector, diagnostics surface and empty/error states.
- Generated dependencies, distribution and coverage are locally ignored.

## Verification

- Python regression: 37 tests passed before onboarding implementation.
- Onboarding unit slice: 5 tests passed.
- Frontend: 2 tests passed.
- TypeScript/Vite production build passed.
- npm audit reported zero vulnerabilities at installation time.

## Open item

VS-014 now has a command-driven onboarding domain and completeness gate. It
still needs its Vue confirmation surface and a Williams mapping flow before it
can be presented as ready for its human gate.
