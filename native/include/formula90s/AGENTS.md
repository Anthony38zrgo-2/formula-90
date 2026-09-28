# include/formula90s — Bridge public surface

Scope: public C++ headers of the Godot bridge, one subfolder per subsystem.
Consumers: `native/src/` implementations and the Rust crates behind the FFI boundary.
Rules: headers are contracts. A behavior change ships with its implementation and test in the same change.
Subfolders: each child folder documents itself in its own AGENTS.md.
