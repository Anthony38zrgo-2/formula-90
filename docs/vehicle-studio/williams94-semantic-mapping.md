# Williams 1994 Semantic Mapping Worksheet

Backlog item: `VS-006`

Status: `ACCEPTED`

Machine-readable proposal:
`tools/vehicle_studio/fixtures/williams94_semantic_mapping_proposed.json`

## 1. Proposed authoring authority

Use the modular resources as Vehicle Studio authoring sources:

```text
F1_94_chassis_core_geometry.glb
F1_94_front_wing_nose_geometry.glb
F1_94_rear_wing_geometry.glb
F1_94_wheel_front_geometry.glb
F1_94_wheel_rear_geometry.glb
```

Treat `F1_94_chassis_geometry.glb` as a compatibility assembly duplicated by
the modular core/front/rear set. It remains a read-only comparison artifact and
is not mapped twice into the authoring scene.

Coverage across all six GLB files:

- primitive occurrences inspected: `46`;
- canonical authoring occurrences mapped: `27`;
- compatibility duplicate occurrences ignored with reason: `19`;
- unclassified primitive occurrences: `0`.

## 2. Source-explicit mappings

The following roles are directly supported by mesh names and module identity:

- chassis and body interior;
- nose;
- front wing;
- rear wing;
- cockpit LCD objects;
- driver helmet;
- suspension FL/FR/RL/RR;
- canonical front wheel hub, inner/outer tire and tread;
- canonical rear wheel hub, inner/outer tire and tread.

Vehicle origin, both axle centers, four wheel anchors, front/rear wing mounts
and nose/chassis interface are source-explicit nodes with zero matrix delta in
the modular split report.

## 3. Embedded regions requiring onboarding confirmation

`GEO_CHASSIS` contains sidepods, engine cover, floor, cockpit shell and
diffuser-related geometry in one primitive. Reports describe prior selections
but do not preserve current persistent face/vertex IDs.

Consequently:

- sidepod-left and sidepod-right bindings require interactive confirmation;
- engine-cover binding requires interactive confirmation;
- cockpit, floor, airbox crown and rear-suspension low zone require protected
  bindings;
- the sidepod report's 836 original faces are historical evidence, not a safe
  current selection;
- the body report's 65 support and 23 affected engine-cover faces are likewise
  historical evidence only.

No coordinate heuristic is promoted to persistent authority by this worksheet.

## 4. Nose and wings

The nose is an independent mesh and may use the whole primitive as its initial
deformation region, but its fixed front-wing and chassis interface vertex sets
still require confirmation.

Front and rear wings are independent meshes. Whole-object span/position/
incidence transforms are safe initial capabilities. Per-element chord or
relative-element editing remains unavailable until element-level bindings are
confirmed; v1 must not infer those groups silently.

## 5. Human-gate decision

Acceptance of this worksheet approves:

1. the five modular GLBs as authoring authority;
2. the full chassis GLB as an ignored compatibility duplicate;
3. all source-explicit component and frame roles;
4. interactive mapping as mandatory for persistent deform/protected regions;
5. whole-object-only wing editing until element bindings exist;
6. creation of revision zero only after the later onboarding UI records those
   confirmations.

Acceptance does not approve guessed vertex selections, historical
reproducibility or visual quality.
