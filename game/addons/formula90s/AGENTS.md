# formula90s — Owned bridge plugin

Scope: the Formula90 GDExtension plugin: manifest, compiled libraries, and sky shaders.
Consumers: the Godot project at editor and runtime startup.
Rules: the `.gdextension` manifest is the load contract. Binaries and shaders change only through the native and Rust builds, never by hand. The `bin/` build-output folder is ignore-blocked from carrying its own file and is covered here: tracked versioned libraries, rebuild-safety applies, BUILD must match HEAD.
Subfolders: the `shaders/` folder documents itself in its own AGENTS.md.

The physical-world bridge requires the rebuilt native extension and a compatible physics library exporting the full source identifier. Keep the core presentation snapshot capability available without changing ABI 17 layout. After a rebuild, compare installed Rust libraries with the final Cargo products; verify both canonical and debug-template copies. Record dirty-source provenance as well as BUILD/HEAD parity.
