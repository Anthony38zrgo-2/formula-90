# Formula-90 Agent Protocol

CANON SCRIPT:

`run_f1_94.ps1`

PIPELINE:

plan sprint -> select backlog item -> implement -> review -> human approval -> done

SCOPE: If scope, intent, or affected systems are unclear, stop and request human clarification. Never infer requirements.

CODE: Use fully self-explanatory names. Abbreviations are prohibited in identifiers, filenames, variables, functions, classes, methods, fields, and new symbols. Do not add code comments. Code must explain itself through naming and structure.

PROVENANCE: Before any change, verify and record branch, HEAD, and git status. Never switch, reset, or alter a dirty branch before inventorying its changes and creating an approved backup.

COMMIT SCOPE: Keep commits atomic. Stage explicit paths only. `git add -A` is prohibited. Never commit pre-existing, generated, unrelated, or foreign changes. Verify `git diff --cached` before commit.

REBUILD SAFETY: Never reuse DLLs, object files, or ignored caches across branches. For full rebuilds, verify HEAD and clean Cargo, SCons, and `game/.godot`. The resulting BUILD must match the current source.

RUNTIME PARITY: Runtime must reject binaries whose BUILD does not match HEAD. Launch scripts must compile required binaries or validate BUILD/HEAD parity before starting Godot.

GLOSSARY:

- BUILD: the source snapshot recorded in `game/BUILD_SOURCE`; runtime parity compares it against HEAD.
- promotion: moving approved content from `scratch/` into tracked homes (`game/assets/`, `game/sounds/`), enforced by `tools/common/output_policy.py`.
- binary leaf: a folder holding only media with no code, config, or scene files; its parent AGENTS.md documents it.
- ignore-blocked: a `.gitignore` rule blocks new files in a folder, so it cannot carry its own AGENTS.md; its parent documents it.
- golden: a committed baseline (values, hashes, telemetry) that tests compare against; intentional changes update goldens explicitly in the same change.
