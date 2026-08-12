"""Restricted F90 Track SVG profile.

Defines the canonical, untrusted-input boundary for the Track Authoring System.

Conventions
-----------
* 1 SVG unit == 1 metre (no CSS pixels, no viewport scaling).
* The `viewBox` is immutable and authored in world metres.
* Axis mapping is identity: SVG x -> world X (Godot X), SVG y -> world Z (Godot Z).
  Terrain-grid winding and the Blender conversion are owned by ``terrain_grid``.
* Semantic payload travels in `data-*` attributes. Structural geometry uses a
  strict whitelist of attributes per element.
* No scripts, handlers, references, styles, filters, animations, fonts or
  unknown namespaces survive sanitization.
"""

from __future__ import annotations

from pathlib import Path
import hashlib
import re

SVG_NS = "http://www.w3.org/2000/svg"
F90_NS = "urn:formula90:track"

PROFILE_VERSION = "f90-track-1"
SCHEMA_VERSION = 1

ROOT_LOCAL_NAME = "svg"
ROOT_LOCAL_NAMES = {"svg", "metadata", "g", "path", "polygon", "polyline", "line", "circle", "rect"}
# Namespace-qualified ElementTree tag is `{SVG_NS}name`; F90 metadata children are `{F90_NS}name`.
ALLOWED_NAMESPACES = {SVG_NS, F90_NS}

# Elements permitted by the restricted profile.
ALLOWED_ELEMENTS = {"svg", "metadata", "g", "path", "polygon", "polyline", "line", "circle", "rect"}

# Structural (non data-*) attributes allowed, keyed by element local name.
ALLOWED_ATTRIBUTES: dict[str, set[str]] = {
    "svg": {"viewBox", "width", "height", "version"},
    "g": set(),
    "path": {"d"},
    "polygon": {"points"},
    "polyline": {"points"},
    "line": {"x1", "y1", "x2", "y2"},
    "circle": {"cx", "cy", "r"},
    "rect": {"x", "y", "width", "height"},
    "metadata": set(),
}

DATA_PREFIX = "data-"

# Any element/attribute on this list (or any `on*` handler, or any `style`/
# `filter`/`href`/`xlink`/`script` reference) is rejected with an actionable error.
FORBIDDEN_SUBSTRINGS = (
    "script",
    "foreignobject",
    "iframe",
    "audio",
    "video",
    "image",
    "use",
    "style",
    "filter",
    "animation",
    "animate",
    "font",
)

# The full path grammar we can flatten. Unsupported commands are rejected during
# canonicalization so compiled geometry never depends on browser tessellation.
ALLOWED_PATH_COMMANDS = set("MmLlHhVvCcQqZz")

# Stable per-instance asset identifiers live in the canonical SVG. They must
# never be derived from DOM order during normalization.
INSTANCE_ID_RE = re.compile(r"^[a-z0-9][a-z0-9_.-]{0,63}$")

# Track identifiers name filesystem directories and Git-scoped commits; they
# must be safe path components and never contain separators or traversal.
TRACK_ID_RE = re.compile(r"^[a-z0-9][a-z0-9_.-]{0,63}$")

# Numeric `data-*` attributes owned by the profile. Their values must parse as
# finite floats so compiled geometry is never NaN/Inf-dependent.
NUMERIC_DATA_ATTRIBUTES = frozenset({
    "data-s-m",
    "data-degrees",
    "data-height-m",
    "data-scale",
    "data-yaw-rad",
    "data-width-m",
    "data-road-surface-elevation-m",
    "data-spacing-m",
    "data-target-count",
})

# Roles owned by the restricted profile. The sanitizer passes `data-*` through,
# but normalization keys off these roles to build the compiled contract.
ASSET_ROLE = "asset-instance"
TERRAIN_ROLE = "terrain-zone"
VEGETATION_REGION_ROLE = "vegetation-region"
VEGETATION_BOUNDARY_ROLE = "vegetation-boundary"
VEGETATION_GENERATED_ATTR = "data-generated-by-region"
VEGETATION_CATEGORY_ATTR = "data-category"

# Stable region identifiers (safe path/commit components like track ids).
REGION_ID_RE = re.compile(r"^[a-z0-9][a-z0-9_.-]{0,63}$")

# Authoring control ranges (metres / degrees). Values outside these ranges are
# rejected during normalization with actionable diagnostics.
BANKING_RANGE_DEG = (-45.0, 45.0)
ELEVATION_HEIGHT_RANGE_M = (-25.0, 25.0)


def local_name(tag: str) -> str:
    """Return the local tag name from an ElementTree tag (strips namespace)."""
    return tag.rsplit("}", 1)[-1] if tag.startswith("{") else tag


def tag_namespace(tag: str) -> str:
    if tag.startswith("{") and "}" in tag:
        return tag[1:].split("}", 1)[0]
    return ""


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def is_data_attribute(name: str) -> bool:
    return name.startswith(DATA_PREFIX)
