# scripts — Canon entry points

Scope: PowerShell and shell launchers, builders, bootstrappers, and test runners for every platform, including the canon `run_f1_94.ps1`.
Consumers: developers and automation starting builds, tests, and the Godot runtime.
Rules: entry points only. Implementation lives in `tools/` and `game/`; never duplicate build logic here.

## Physical-world launch and rebuild

`run_f1_94.ps1 -PhysicalWorldPackage <absolute-package-path>` explicitly selects the candidate for a normal interactive session. Preserve its incompatibility checks with smoke, diagnostic, parity and showcase modes. Both livery variants retain the existing asset, profile-digest and BUILD/HEAD checks.

For a full rebuild, record branch, HEAD and dirty inventory; preserve modified sources and current binaries before cleaning Cargo, SCons, the DSP build and `game/.godot`. Use `build_windows.ps1` for the build. Verify final Cargo products against both installed canonical and template DLLs after all crates finish. Copy errors must be resolved and hashes verified before launching.

Verify DSP Debug/Release tests and installed binary hashes, then perform a fresh import and canonical smoke. Record uncommitted source changes separately from the HEAD tag: BUILD/HEAD equality alone does not prove a clean source snapshot. Generated binaries, cache files and `BUILD_SOURCE` are excluded from source commits.
