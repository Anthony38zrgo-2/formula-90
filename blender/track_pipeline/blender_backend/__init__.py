"""Blender materialization backend for BuildIR.

This package consumes ``track.build.json`` (BuildIR) produced by the Rust
``track-build`` compiler and materializes it in Blender. It is the Python/bpy
half of the TS-110 boundary: semantic decisions and deterministic geometry are
owned by Rust; this package only creates meshes, assigns materials, instances
objects and exports GLB/.blend.

The pure (non-bpy) modules here are unit-testable without Blender. Any module
that imports ``bpy`` does so lazily so the package can be imported and tested in
a plain Python interpreter.
"""
