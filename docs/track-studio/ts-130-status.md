# TS-130 Frontend TypeScript Status

## Result

The frontend presentation layer is implemented in TypeScript under:

```text
tools/track_studio/app/src/ts/
```

It is framework-free, modular and typed against `@tauri-apps/api`. It talks to
Rust only through the typed IPC surface in `core/api.ts`; it holds no SQL, no
domain geometry and no validation rules.

## Modules

- `core/api.ts`: typed Tauri IPC wrapper (project_open/save, snapshot,
  validate, undo/redo, compile_build_ir, build_plan).
- `core/state.ts`: minimal UI state store with pub/sub.
- `core/selection.ts`: viewport selection set.
- `viewport/viewport.ts`: plan 2D surface host + active tool.
- `tools/select-tool.ts`: `Tool` interface and a select tool.
- `panels/problems-panel.ts`: renders diagnostics.
- `panels/build-panel.ts`: compiles BuildIR and shows the incremental plan.
- `ui/app.ts`: toolbar wiring and bootstrap.
- `index.html`: toolbar, viewport and side panels; loads `./js/ui/app.js`.

## Build

```text
npm install                       PASS
npm run typecheck                 PASS (tsc --noEmit, strict)
npm run build                     PASS (emits app/src/js/*.js)
```

## Explicit limitations

- This is the modular presentation shell; editing tools, snapping and the
  profile views are still placeholder/minimal (`TS-140`).
- The Tauri app must be rebuilt to embed the new frontend assets for a desktop
  run.

Next is the real authoring tooling and reference-image workflow (`TS-140`).
