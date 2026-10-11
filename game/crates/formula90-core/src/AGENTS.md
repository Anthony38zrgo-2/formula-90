# src — Core implementation

Scope: core library sources, modules, and diagnostic binaries.
ABI 19 transports up to 32 raw rigid contacts per underfloor sample. Rust owns
contact eligibility, hysteresis, physical load and scrape intensity. Contact
configuration comes from the vehicle JSON through the typed physics profile;
native creation options do not override it. Legacy profiles without the section
retain Rust defaults. Coordinated native and Rust rebuilds are mandatory.
Consumers: downstream crates linking the core library.
Rules: module boundaries in `modules/` are the extension seam; keep them stable. The `bin/` diagnostic-binary folder is ignore-blocked from carrying its own file and is covered here: diagnostics only, never gameplay entry points.

`accept_physical_world_snapshot` validates entity identity, physical time and operating mass before publishing through the existing frame pipeline. Service snapshots at unchanged time must not advance another clock. Keep fuel, driving aids, thermal state and audio consumers aligned with the authoritative snapshot.

The additive `f90_core_accept_physical_world_snapshot` entry point retains ABI 17 and its output layout. Preserve buffer, document-size and panic-boundary checks. A breaking layout change requires coordinated native and Rust rebuilds and an explicit ABI revision.
