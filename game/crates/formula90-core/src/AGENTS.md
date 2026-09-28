# src — Core implementation

Scope: core library sources, modules, and diagnostic binaries.
Consumers: downstream crates linking the core library.
Rules: module boundaries in `modules/` are the extension seam; keep them stable. The `bin/` diagnostic-binary folder is ignore-blocked from carrying its own file and is covered here: diagnostics only, never gameplay entry points.
