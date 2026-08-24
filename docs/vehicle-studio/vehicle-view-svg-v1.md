# VehicleView SVG v1 Contract

Backlog item: `VS-005`

Status: `REVIEWED`

Machine-readable profile:
`tools/vehicle_studio/schemas/vehicle_view_profile_v1.json`

## 1. Authority

VehicleView SVG is a deterministic projection and semantic command surface. It
is not independent vehicle geometry and cannot create topology. VehicleDocument
remains authoritative.

One SVG user unit equals one metre in the declared orthographic view. The root
identifies project, revision, source hash, document hash, view and profile.

## 2. Editable elements

Every editable element has a persistent `id`, a declared editable role and an
`f90:binding` metadata entry. A deformation control additionally references a
VehicleDocument parameter and a model axis.

Moving a control emits the declared semantic command. The client sends that
command to the local service; it does not save the mutated SVG DOM. A new SVG is
compiled from the resulting VehicleDocument revision.

## 3. Safety profile

The profile forbids script, style, event attributes, foreign objects, links,
images, animation and external resources. Styling is provided by the trusted
application outside the persisted SVG.

Unknown elements, namespaces, attributes, roles or editable bindings fail
sanitization. They are not dropped silently.

## 4. Canonicalization

- UTF-8 without XML declaration.
- LF newlines.
- Six decimal places and normalized negative zero.
- Lexicographic attribute ordering.
- Semantic group order from the machine-readable profile.
- Persistent-ID ordering within groups.
- Absolute path commands only.
- No timestamps or absolute filesystem paths.

## 5. Fixtures

| Fixture | Expected result | Diagnostic |
|---|---|---|
| `vehicle_view_valid_minimal.svg` | `PASS` | none |
| `vehicle_view_invalid_script.svg` | `FAIL` | `VEHICLE_VIEW_FORBIDDEN_ELEMENT` |
| `vehicle_view_invalid_external.svg` | `FAIL` | `VEHICLE_VIEW_EXTERNAL_RESOURCE` |
| `vehicle_view_invalid_event.svg` | `FAIL` | `VEHICLE_VIEW_FORBIDDEN_ATTRIBUTE` |
| `vehicle_view_invalid_unbound_control.svg` | `FAIL` | `VEHICLE_VIEW_CONTROL_BINDING` |

The executable sanitizer and byte-stable compiler belong to later backlog
items. VS-005 freezes the contract and expected fixture outcomes.

