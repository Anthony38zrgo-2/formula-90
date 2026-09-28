# native — GDExtension bridge

Scope: C++ glue between Godot and the Rust simulation workspace. Public contracts live under `include/`, implementations under `src/`, C++ tests under `tests/`.
Consumers: the SCons build producing the libraries in `game/addons/formula90s/bin/`, loaded through `formula90s.gdextension`.
Rules: root rebuild-safety and runtime-parity rules apply. Never ship a binary whose BUILD does not match HEAD.
Subfolders: each child folder documents itself in its own AGENTS.md.
