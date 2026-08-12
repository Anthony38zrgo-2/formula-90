"""Shared provenance and GLB helpers for the normalized JSON Blender build.

This module must stay importable both inside a Blender headless process and
inside the plain track-pipeline virtualenv, so it must not import ``bpy``.

It owns:
* the compiler name/version and its deterministic file-content SHA-256;
* the deterministic ``build_sha256`` used for ``builds/<build_sha>/``;
* pure-Python GLB inspection (magic/JSON chunk + node names) and the runtime
  environment/vegetation split check.
"""

from __future__ import annotations

from pathlib import Path
import hashlib
import json
import struct

COMPILER_NAME = "build_normalized_track_blender"
COMPILER_VERSION = "1.0.0"

_COMPILER_MODULES = (
    "build_normalized_track_blender.py",
    "build_normalized_common.py",
    "terrain_grid.py",
    "blender_output.py",
    "procedural_materials_blender.py",
    "build_track_blender.py",
    "asset_registry.py",
)

BUILD_MANIFEST_FILE = "build.manifest.json"
BUILD_MANIFEST_SCHEMA_VERSION = 1

BLEND_FILE = "track.blend"
ENVIRONMENT_GLB = "track_runtime_environment.glb"
VEGETATION_GLB = "track_runtime_vegetation.glb"


class BuildManifestError(ValueError):
    """Raised when a build manifest or GLB split contract is violated."""


def compiler_sha256() -> str:
    """SHA-256 over compiler module file contents (deterministic per checkout)."""
    digest = hashlib.sha256()
    base = Path(__file__).resolve().parent
    for name in _COMPILER_MODULES:
        digest.update(name.encode("utf-8"))
        digest.update((base / name).read_bytes())
    return digest.hexdigest()


def build_sha256(*hashes: str) -> str:
    """Deterministic build id: SHA-256 over the four input hashes."""
    digest = hashlib.sha256()
    for item in hashes:
        digest.update(str(item).encode("ascii"))
        digest.update(b"\x00")
    return digest.hexdigest()


def glb_node_names(path: str | Path) -> set[str]:
    """Return the node names stored in a GLB (validates magic + JSON chunk)."""
    path = Path(path)
    with path.open("rb") as handle:
        if handle.read(4) != b"glTF":
            raise BuildManifestError(f"Invalid GLB magic: {path}")
        handle.read(8)
        json_length = struct.unpack("<I", handle.read(4))[0]
        if handle.read(4) != b"JSON":
            raise BuildManifestError(f"GLB JSON chunk missing: {path}")
        document = json.loads(handle.read(json_length).decode("utf-8").rstrip("\x00 \t\r\n"))
    return {str(node.get("name", "")) for node in document.get("nodes", [])}


def validate_runtime_split(
    environment_glb: str | Path,
    vegetation_glb: str | Path,
    collision_names: set[str],
    asset_names: set[str] | None = None,
) -> None:
    """Assert the physical runtime owns every collision proxy and the
    vegetation runtime owns every collision-free asset root."""
    environment_names = glb_node_names(environment_glb)
    vegetation_names = glb_node_names(vegetation_glb)
    missing = sorted(collision_names - environment_names)
    leaked = sorted(collision_names & vegetation_names)
    if missing:
        raise BuildManifestError(f"Physical runtime omitted collision proxies: {missing}")
    if leaked:
        raise BuildManifestError(f"Vegetation runtime contains collision proxies: {leaked}")
    if asset_names is not None:
        absent = sorted(asset_names - vegetation_names)
        if absent:
            raise BuildManifestError(f"Vegetation runtime omitted asset roots: {absent}")
