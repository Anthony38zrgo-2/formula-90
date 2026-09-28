# Formula-90 Game Canon

This file is binding for every change under `game/`. It specializes the root
`AGENTS.md` protocol for the Godot project.

## Project layout

- Runtime audio truth lives in `game/sounds/` and is governed by
  `game/sounds/AGENTS.md`, including the canonical V10 engine bank.
  The historical sample tree `game/audio/` is deprecated; never create a
  runtime bank there.
- Simulation truth lives in Rust under `game/crates/`, bridged through
  `native/` and `game/addons/formula90s/`. Tuning truth lives in data
  under `game/data/`; scenes under `game/scenes/` compose, and scripts
  under `game/scripts/` wire behavior.
- Tests live in `game/tests/` for Godot suites, beside Rust `tests/`
  folders per crate and Python suites under `tools/*/tests/`.

## Project-wide rules

- The Rust runtime loads bank audio from the filesystem, so bank assets are
  never referenced through `res://` paths. Folders carrying a `.gdignore`
  are not imported by Godot by design.
- Regeneration beats hand-editing: derived artifacts rebuild from sources
  through `tools/`; promotion follows `tools/common/output_policy.py`.
- Each folder documents its own domain in its own `AGENTS.md`. A parent
  file names the domain boundary; child detail lives with the child.
  Deliberate exceptions (vendored code, binary leaves, ignore-blocked
  folders) are recorded in `docs/folder-domain-audit.md`.
