# formula90s — Owned bridge plugin

Scope: the Formula90 GDExtension plugin: manifest, compiled libraries, and sky shaders.
Consumers: the Godot project at editor and runtime startup.
Rules: the `.gdextension` manifest is the load contract. Binaries and shaders change only through the native and Rust builds, never by hand. The `bin/` build-output folder is ignore-blocked from carrying its own file and is covered here: tracked versioned libraries, rebuild-safety applies, BUILD must match HEAD.
Subfolders: the `shaders/` folder documents itself in its own AGENTS.md.
