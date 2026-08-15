"""Blender export helpers for the materialization backend.

These functions require an active Blender context (``bpy``) and are used by the
orchestrator to emit the GLB runtime parts and the optional inspection ``.blend``
file from the materialized scene.
"""

from __future__ import annotations

from pathlib import Path
from typing import Iterable, Any


def export_glb(objects: Iterable[Any], path: str | Path) -> None:
    """Export the given objects as a GLB file (selected-only export)."""
    import bpy

    bpy.ops.object.select_all(action="DESELECT")
    selected = 0
    for obj in objects:
        if obj.hide_get():
            continue
        obj.select_set(True)
        selected += 1
    if selected == 0:
        raise RuntimeError(f"no visible objects to export to {path}")
    bpy.ops.export_scene.gltf(
        filepath=str(path),
        export_format="GLB",
        use_selection=True,
    )


def save_blend(path: str | Path) -> None:
    """Save the current scene as a Blender file."""
    import bpy

    bpy.ops.wm.save_as_mainfile(filepath=str(path))
