"""Sanitize and canonicalize an untrusted SVG into the F90 Track SVG profile.

Rejects unsafe/unsupported content with actionable diagnostics and returns a
canonical ``ElementTree`` whose serialization is deterministic (sorted
attributes, no stray namespaces, stable structural ordering, no transforms).
"""

from __future__ import annotations

from typing import Any
from xml.etree import ElementTree as ET
import math

from svg_profile import (
    ALLOWED_ATTRIBUTES,
    ALLOWED_ELEMENTS,
    ALLOWED_NAMESPACES,
    ALLOWED_PATH_COMMANDS,
    DATA_PREFIX,
    F90_NS,
    FORBIDDEN_SUBSTRINGS,
    INSTANCE_ID_RE,
    NUMERIC_DATA_ATTRIBUTES,
    PROFILE_VERSION,
    SVG_NS,
    is_data_attribute,
    local_name,
    sha256_bytes,
    tag_namespace,
)


class SVGSanitizeError(ValueError):
    """Raised when input SVG cannot be accepted into the restricted profile."""

    def __init__(self, diagnostics: list[str]):
        self.diagnostics = diagnostics
        super().__init__("; ".join(diagnostics))


def _element_id(context: str, index: int, tag: str) -> str:
    return f"{context}-{index:04d}"


# Parameter count per path command; Z takes no parameters.
_PATH_COMMAND_PARAMS = {
    "M": 2, "m": 2, "L": 2, "l": 2,
    "H": 1, "h": 1, "V": 1, "v": 1,
    "C": 6, "c": 6, "Q": 4, "q": 4,
    "Z": 0, "z": 0,
}

# Structural numeric attributes per element (finite check).
_STRUCTURAL_NUMERIC = {
    "circle": ("cx", "cy", "r"),
    "line": ("x1", "y1", "x2", "y2"),
    "rect": ("x", "y", "width", "height"),
}


def sanitize_svg(xml_bytes: bytes) -> ET.Element:
    """Return a canonical F90 Track SVG ElementTree or raise ``SVGSanitizeError``."""
    diagnostics: list[str] = []
    try:
        raw = ET.fromstring(xml_bytes)
    except ET.ParseError as exc:
        raise SVGSanitizeError([f"malformed XML: {exc}"]) from exc

    root_ns = tag_namespace(raw.tag)
    if root_ns and root_ns not in ALLOWED_NAMESPACES:
        raise SVGSanitizeError([f"unsupported root namespace: {root_ns}"])

    ET.register_namespace("", SVG_NS)
    ET.register_namespace("f90", F90_NS)

    root = ET.Element(f"{{{SVG_NS}}}svg")
    _reject_attributes(raw, "svg", diagnostics)

    viewbox = raw.get("viewBox")
    if not viewbox:
        diagnostics.append("svg: missing required immutable viewBox")
    else:
        tokens = viewbox.replace(",", " ").split()
        if len(tokens) != 4:
            diagnostics.append(f"svg: viewBox must have 4 numbers, got {viewbox!r}")
        else:
            try:
                [float(v) for v in tokens]
            except ValueError as exc:
                diagnostics.append(f"svg: viewBox not numeric: {viewbox!r} ({exc})")
            root.set("viewBox", viewbox)

    allowed_data = _collect_data_attributes(raw)
    for key in sorted(allowed_data):
        root.set(key, allowed_data[key])

    index = 0
    for child in list(raw):
        _walk(child, root, index, diagnostics)
        index += 1

    if diagnostics:
        raise SVGSanitizeError(diagnostics)

    _assign_instance_ids(root, diagnostics)
    if diagnostics:
        raise SVGSanitizeError(diagnostics)

    root.set("data-profile-version", PROFILE_VERSION)
    root.set("data-source-sha256", sha256_bytes(xml_bytes))
    return root


def _assign_instance_ids(root: ET.Element, diagnostics: list[str]) -> None:
    """Persist a stable ``data-instance-id`` on every asset instance.

    Authored IDs are preserved verbatim; missing IDs are assigned once during
    canonicalization (document order at the import boundary). The canonical SVG
    then owns the IDs, so later normalization never derives them from DOM order.
    Duplicate or malformed IDs are rejected with actionable diagnostics.
    """
    seen: dict[str, str] = {}
    index = 0
    for element in root.iter():
        if element.get("data-role") != "asset-instance":
            continue
        instance_id = element.get("data-instance-id")
        if instance_id is None:
            element.set("data-instance-id", f"asset_{index:04d}")
            index += 1
            continue
        if not INSTANCE_ID_RE.match(instance_id):
            diagnostics.append(
                f"asset-instance: invalid data-instance-id {instance_id!r} "
                "(use [a-z0-9][a-z0-9_.-]*, max 64 characters)"
            )
        if instance_id in seen:
            diagnostics.append(f"asset-instance: duplicate data-instance-id {instance_id!r}")
        seen[instance_id] = element.get("data-asset-id", "")


def _collect_data_attributes(element: ET.Element) -> dict[str, str]:
    return {name: value for name, value in element.attrib.items() if is_data_attribute(name)}


def _reject_attributes(element: ET.Element, context: str, diagnostics: list[str]) -> None:
    allowed_structural = ALLOWED_ATTRIBUTES.get(context, set())
    for name, value in element.attrib.items():
        if is_data_attribute(name):
            continue
        lname = local_name(name)
        lowered = (name + "=" + value).lower()
        if name.startswith("on") or any(part in lowered for part in FORBIDDEN_SUBSTRINGS):
            diagnostics.append(f"{context}: rejected attribute {name!r}")
        elif name in ("transform", "style", "filter", "class"):
            diagnostics.append(f"{context}: attribute {name!r} is not allowed in the restricted profile")
        elif name in ("href", "xlink:href", "id") or lname in ("href", "src"):
            diagnostics.append(f"{context}: external reference {name!r} is not allowed")
        elif name not in allowed_structural:
            diagnostics.append(f"{context}: unexpected attribute {name!r}")


def _walk(source: ET.Element, parent: ET.Element, index: int, diagnostics: list[str]) -> None:
    tag = local_name(source.tag)
    ns = tag_namespace(source.tag)

    if ns and ns not in ALLOWED_NAMESPACES:
        diagnostics.append(f"element {tag!r}: unsupported namespace {ns}")
        return
    if ns == F90_NS:
        # Author-controlled metadata namespace: emit verbatim (safe, no geometry).
        new = ET.Element(f"{{{F90_NS}}}{tag}")
        parent.append(new)
        for name, value in source.attrib.items():
            if is_data_attribute(name):
                new.set(name, value)
        for child in list(source):
            _walk(child, new, index, diagnostics)
        return

    if tag not in ALLOWED_ELEMENTS:
        lowered = tag.lower()
        if any(part in lowered for part in FORBIDDEN_SUBSTRINGS) or tag == "foreignObject":
            diagnostics.append(f"element {tag!r}: rejected unsafe/unsupported element")
        else:
            diagnostics.append(f"element {tag!r}: not in the restricted F90 profile")
        return

    _check_numeric_values(source, tag, diagnostics)
    if tag == "path":
        _check_path_syntax(source, tag, diagnostics)
    elif tag in ("polygon", "polyline"):
        _check_points_syntax(source, tag, diagnostics)

    _reject_attributes(source, tag, diagnostics)

    new = ET.Element(f"{{{SVG_NS}}}{tag}")

    data = _collect_data_attributes(source)
    for key in sorted(data):
        new.set(key, data[key])

    structural = {local_name(k): v for k, v in source.attrib.items() if not is_data_attribute(k)}
    for key in sorted(structural):
        if key in ALLOWED_ATTRIBUTES.get(tag, set()):
            new.set(key, structural[key])

    if tag == "g":
        new.set("data-element-id", _element_id("layer", index, tag))

    parent.append(new)

    child_index = 0
    for child in list(source):
        _walk(child, new, child_index, diagnostics)
        child_index += 1


def _check_path_syntax(element: ET.Element, tag: str, diagnostics: list[str]) -> None:
    d = element.get("d", "")
    if not d:
        return
    tokens = _tokenize_path(d)
    first_letter = next((t for t in tokens if t[0] in "MmLlHhVvCcQqZz"), None)
    if first_letter not in "Mm":
        diagnostics.append(f"{tag}: path must start with an M/m moveto command")

    seen: set[str] = set()
    command = None
    count = 0
    for token in tokens:
        if token[0] in "MmLlHhVvCcQqZz":
            if command is not None:
                _check_command_params(tag, command, count, diagnostics)
            command = token[0]
            count = 0
            seen.add(command)
        else:
            count += 1
            try:
                value = float(token)
            except ValueError:
                diagnostics.append(f"{tag}: malformed path number {token!r}")
            else:
                if not math.isfinite(value):
                    diagnostics.append(f"{tag}: non-finite path number {token!r}")
    if command is not None:
        _check_command_params(tag, command, count, diagnostics)

    unsupported = seen - set(ALLOWED_PATH_COMMANDS)
    if unsupported:
        diagnostics.append(f"{tag}: unsupported path command(s): {''.join(sorted(unsupported))}")


def _check_command_params(tag: str, command: str, count: int, diagnostics: list[str]) -> None:
    step = _PATH_COMMAND_PARAMS[command]
    if step == 0:
        if count:
            diagnostics.append(f"{tag}: command {command!r} must not take parameters (got {count})")
        return
    if count == 0:
        diagnostics.append(f"{tag}: command {command!r} is missing its parameters")
    elif count % step:
        diagnostics.append(f"{tag}: command {command!r} has {count} parameters; expected a multiple of {step}")


def _check_points_syntax(element: ET.Element, tag: str, diagnostics: list[str]) -> None:
    points = element.get("points", "").replace(",", " ").split()
    if not points:
        return
    if len(points) % 2 != 0:
        diagnostics.append(f"{tag}: points must have an even number of coordinates (got {len(points)})")
    for token in points:
        try:
            value = float(token)
        except ValueError:
            diagnostics.append(f"{tag}: malformed points coordinate {token!r}")
        else:
            if not math.isfinite(value):
                diagnostics.append(f"{tag}: non-finite points coordinate {token!r}")


def _check_numeric_values(element: ET.Element, tag: str, diagnostics: list[str]) -> None:
    for name, value in element.attrib.items():
        if is_data_attribute(name):
            if name in NUMERIC_DATA_ATTRIBUTES and _unparseable_number(value):
                diagnostics.append(f"{tag}: attribute {name} must be a finite number, got {value!r}")
            continue
        if tag == "svg" and name == "viewBox":
            for token in value.replace(",", " ").split():
                if _unparseable_number(token):
                    diagnostics.append(f"{tag}: viewBox must contain only finite numbers, got {token!r}")
        elif name in _STRUCTURAL_NUMERIC.get(tag, ()) and _unparseable_number(value):
            diagnostics.append(f"{tag}: attribute {name} must be a finite number, got {value!r}")


def _unparseable_number(raw: str) -> bool:
    try:
        return not math.isfinite(float(raw))
    except ValueError:
        return True


def _tokenize_path(d: str) -> list[str]:
    tokens: list[str] = []
    number = ""
    for char in d.replace("\n", " ").replace(",", " "):
        if char in "MmLlHhVvCcQqZz":
            if number.strip():
                tokens.extend(number.strip().split())
                number = ""
            tokens.append(char)
        else:
            number += char
    if number.strip():
        tokens.extend(number.strip().split())
    return tokens


def canonical_xml(root: ET.Element) -> bytes:
    """Serialize the canonical tree deterministically."""
    return ET.tostring(root, encoding="utf-8", xml_declaration=True)
