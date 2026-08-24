# VS-023 — Linked SVG view panels

Status: READY_FOR_HUMAN_GATE

## Outcome

The top, side, front and rear CAD panels now share a single component
selection. Clicking a semantic SVG path highlights the matching component in
all four views and dims unrelated components.

Selection emits a semantic select_component command containing the persistent
component ID. The SVG DOM remains presentation-only and is never persisted as
project authority.

## Security and determinism

Before insertion, every server SVG passes a strict allowlist sanitizer:

- allowed elements: svg, title, g and path;
- executable elements and event attributes are removed;
- only CAD geometry, provenance, styling and semantic ID attributes survive;
- selecting or clearing a component does not mutate canonical SVG bytes.

## Verification

- synchronized selection is verified across four panels;
- semantic command emission is verified;
- malicious script/event SVG fixtures are stripped;
- 4 Vue tests pass;
- the production TypeScript/Vite build passes.
