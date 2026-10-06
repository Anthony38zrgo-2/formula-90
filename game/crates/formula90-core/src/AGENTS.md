# src — Core implementation

Scope: core library sources, modules, and diagnostic binaries.
Consumers: downstream crates linking the core library.
Rules: module boundaries in `modules/` are the extension seam; keep them stable. The `bin/` diagnostic-binary folder is ignore-blocked from carrying its own file and is covered here: diagnostics only, never gameplay entry points.

`accept_physical_world_snapshot` validates entity identity, physical time and operating mass before publishing through the existing frame pipeline. Service snapshots at unchanged time must not advance another clock. Keep fuel, driving aids, thermal state and audio consumers aligned with the authoritative snapshot.

The additive `f90_core_accept_physical_world_snapshot` entry point retains ABI 17 and its output layout. Preserve buffer, document-size and panic-boundary checks. A breaking layout change requires coordinated native and Rust rebuilds and an explicit ABI revision.
