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

GODOT MCP TOOLING (MANDATORY FOR ALL AGENTS): Use the Fulviuus/godot-mcp server for Godot project inspection, scene/resource queries, runtime inspection, screenshots, and Godot debugging whenever the corresponding tools are available. This repository-wide requirement applies to every agent working in any subdirectory. Upstream: https://github.com/Fulviuus/godot-mcp.

- Codex server name: `godot`. Local installation: `C:/Users/bill/.codex/mcp-servers/fulviuus-godot-mcp`; entrypoint: `dist/index.js`. Configured in `C:/Users/bill/.codex/config.toml` with project root `D:/Formula90s/game` and the repository's Godot 4.7.1 binary. Other clients must register this server explicitly; this file does not install or configure their tools.
- Begin Godot work with `godot_project_info` and use the matching resource, scene, log, and screenshot tools to obtain evidence. File inspection and native command-line build/test tools remain necessary for Rust, C++, Python, and repository validation.
- Preserve the canonical launcher `scripts/run_f1_94.ps1` and its BUILD/HEAD checks. The server's `godot_build` only imports/checks Godot resources; it does not rebuild the native Rust/C++ binaries. Do not use `godot_run`, `run_project`, or other direct launch tools to bypass canonical validation or change the intended vehicle session.
- Live tools require an installed bridge and a compatible running session. The native `godot_*` bridge and advanced `game_*` bridge are separate; do not mix their session assumptions. `godot_setup`, `godot_install_bridge`, and live launch tools can modify addons or `project.godot`; inventory those changes and preserve existing plugins/autoloads. Keep debugging bridges inactive in normal gameplay and exports.
- If this server is unavailable or the required live bridge is absent, report that limitation explicitly. Restore the connection where possible; use canonical scripts for independent checks and never claim runtime or visual validation from static inspection alone.

GLOSSARY:

- BUILD: the source snapshot recorded in `game/BUILD_SOURCE`; runtime parity compares it against HEAD.
- promotion: moving approved content from `scratch/` into tracked homes (`game/assets/`, `game/sounds/`), enforced by `tools/common/output_policy.py`.
- binary leaf: a folder holding only media with no code, config, or scene files; its parent AGENTS.md documents it.
- ignore-blocked: a `.gitignore` rule blocks new files in a folder, so it cannot carry its own AGENTS.md; its parent documents it.
- golden: a committed baseline (values, hashes, telemetry) that tests compare against; intentional changes update goldens explicitly in the same change.
