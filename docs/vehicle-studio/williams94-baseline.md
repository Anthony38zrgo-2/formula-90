# Williams 1994 Vehicle Studio Baseline

Backlog item: `VS-001`

Status: `REVIEWED`

Source: `blender/williams94_wheels_retextured/`

Source policy: read-only, currently untracked in the `f1-94` worktree.

## 1. Evidence contract

This baseline covers exactly:

- six source GLBs under `geometry/`;
- 39 PNG files under `textures/albedo/`;
- ten top-level JSON manifests/reports.

Godot `.import` sidecars are generated consumer state and are not part of the
source fingerprint. The canonical fingerprint payload consists of one UTF-8
line per covered file, ordered by repository-relative path:

```text
relative/path<TAB>size_bytes<TAB>uppercase_sha256<LF>
```

Covered files: `55`

Package fingerprint SHA-256:
`3C7B996CFAB2EDF263E223A06846BF84C6822CD343D21E03561CD9AD19FB90A9`

The existing GLB inspector was run twice against the full chassis. Both stdout
payloads hashed to
`DF8024BE6F58053F9ADB841D2966AE1663E117614531328D861E2803FB27F3A4`,
confirming byte-identical repeated evidence for that probe.

## 2. Geometry inventory

All dimensions and bounds use the source convention: metres, `+X` right,
`+Y` up and `-Z` forward.

| Path | SHA-256 | Bytes | Meshes | Nodes | Positions | Triangles | Dimensions X/Y/Z m | UV0 primitives | Normal primitives |
|---|---|---:|---:|---:|---:|---:|---|---:|---:|
| `geometry/F1_94_chassis_core_geometry.glb` | `E4A0DA9642ED9C3E5607C27460BFEF70DF06A264D34B84103C7405B067ACA66C` | 433276 | 16 | 73 | 11790 | 3930 | 1.424440 / 0.924899 / 3.365080 | 16/16 | 1/16 |
| `geometry/F1_94_chassis_geometry.glb` | `5094201C0163C38308AF02EA345EF8A3A9355B66298FB2B2D8204E66F2211B32` | 604164 | 19 | 76 | 22668 | 7556 | 1.424440 / 0.924899 / 4.258080 | 19/19 | 15/19 |
| `geometry/F1_94_front_wing_nose_geometry.glb` | `0C68F2D899F95C73095E8DA5A939586D4C903B27F91E6F7A98DD85273FD9DAD0` | 92676 | 2 | 7 | 2916 | 972 | 1.314246 / 0.411577 / 0.678530 | 2/2 | 1/2 |
| `geometry/F1_94_rear_wing_geometry.glb` | `006BECE9B7F49FE2D851DF6A850F61B43E62D00A8329042A92A6C2A0B8F2BF9D` | 288244 | 1 | 5 | 7962 | 2654 | 0.915623 / 0.792923 / 0.358230 | 1/1 | 1/1 |
| `geometry/F1_94_wheel_front_geometry.glb` | `31AAC43D41A283AED8E5E68BA6F3380EAF8968BD0066AC1C5FAF35CB7C06BA03` | 77308 | 4 | 11 | 2064 | 688 | 0.300299 / 0.633909 / 0.633910 | 4/4 | 4/4 |
| `geometry/F1_94_wheel_rear_geometry.glb` | `62A53860EE233DC3B6FC3A4CDCF0128B091B9050530216F5C78CBED4D03CB6D9` | 77312 | 4 | 11 | 2064 | 688 | 0.368325 / 0.658018 / 0.658018 | 4/4 | 4/4 |

The runtime visual assembly represented by full chassis plus four wheel
instances contains `7,556 + 4 * 688 = 10,308` triangles.

### 2.1 World bounds

| Geometry | Minimum X/Y/Z m | Maximum X/Y/Z m |
|---|---|---|
| Chassis core | -0.712220 / -0.232887 / -1.641881 | 0.712220 / 0.692012 / 1.723199 |
| Full chassis | -0.712220 / -0.232887 / -2.320411 | 0.712220 / 0.692012 / 1.937669 |
| Front wing + nose | -0.657123 / -0.206992 / -2.320411 | 0.657123 / 0.204585 / -1.641881 |
| Rear wing | -0.457612 / -0.161344 / 1.579438 | 0.458011 / 0.631579 / 1.937669 |
| Front wheel local | -0.150149 / -0.316954 / -0.316955 | 0.150149 / 0.316954 / 0.316955 |
| Rear wheel local | -0.184162 / -0.329009 / -0.329009 | 0.184162 / 0.329009 / 0.329009 |

### 2.2 Structural findings

- Every primitive has `POSITION` and `TEXCOORD_0`.
- Front and rear wheel primitives have complete normals.
- Full chassis normals are present on 15 of 19 primitives.
- Chassis-core normals are present on only 1 of 16 primitives.
- No inspected GLB reports negative-scale nodes or negative world
  determinants.
- The source GLBs contain placeholder materials and no embedded images.
- Missing normals are a build/export validation concern; this baseline does not
  mutate or repair them.

## 3. Semantic-node inventory

### 3.1 Vehicle and axle frames

```text
DATUM_VEHICLE_ORIGIN
DATUM_FRONT_AXLE_CENTER
DATUM_REAR_AXLE_CENTER
JNT_WHEEL_FL
JNT_WHEEL_FR
JNT_WHEEL_RL
JNT_WHEEL_RR
JNT_FRONT_WING_MOUNT
JNT_NOSE_CHASSIS
JNT_REAR_WING_MOUNT
JNT_DRIVER_HEAD
```

### 3.2 Suspension frames

Each corner contains `JNT_SUSP_<CORNER>_CHASSIS` and
`JNT_SUSP_<CORNER>_HUB`. Front corners contain five numbered `INNER`/`OUTER`
arm pairs; rear corners contain four numbered pairs.

```text
JNT_SUSP_FL_ARM_01..05_INNER/OUTER
JNT_SUSP_FR_ARM_01..05_INNER/OUTER
JNT_SUSP_RL_ARM_01..04_INNER/OUTER
JNT_SUSP_RR_ARM_01..04_INNER/OUTER
```

### 3.3 Canonical wheel-local frames

Both front and rear canonical wheel GLBs contain:

```text
DATUM_WHEEL_CENTER
DATUM_WHEEL_INBOARD_FACE
DATUM_WHEEL_OUTBOARD_FACE
DATUM_TIRE_CONTACT_PATCH
JNT_WHEEL_MOUNT
```

The front wheel local half-width is `0.1501494944 m` and effective radius is
approximately `0.3169545 m`. The rear wheel local half-width is
`0.1841624975 m` and effective radius is approximately `0.3290090 m`.

## 4. Texture inventory

All 39 PNG files are 64x64. They collapse into five content hashes.

### Group A — body atlas, 19 copies

SHA-256: `FDC055F2A4F24A668558F64FDD09ED83C9DD7A9B6DE70F89F4317744592554B6`

```text
GEO_AERO_FRONT_WING.png
GEO_AERO_REAR_WING.png
GEO_BODY_INTERIOR.png
GEO_CHASSIS.png
GEO_COCKPIT_LCD_06.png
GEO_COCKPIT_LCD_07.png
GEO_COCKPIT_LCD_08.png
GEO_COCKPIT_LCD_09.png
GEO_COCKPIT_LCD_13.png
GEO_COCKPIT_LCD_16.png
GEO_COCKPIT_LCD_17.png
GEO_COCKPIT_LCD_18.png
GEO_COCKPIT_LCD_INDICATOR.png
GEO_DRIVER_HELMET.png
GEO_NOSE.png
GEO_SUSPENSION_FL.png
GEO_SUSPENSION_FR.png
GEO_SUSPENSION_RL.png
GEO_SUSPENSION_RR.png
```

### Group B — wheel hubs, five copies

SHA-256: `0DB9691C2F47CFBDCB9C0D1B00BDE19548DA068CB282611495A4E05D5EF06919`

```text
GEO_WHEEL_FL_HUB.png
GEO_WHEEL_FR_HUB.png
GEO_WHEEL_HUB.png
GEO_WHEEL_RL_HUB.png
GEO_WHEEL_RR_HUB.png
```

### Group C — wheel inner faces, five copies

SHA-256: `72D93FC690894E5A7BE1AE4C5032B84AE7FD6F2BE68B7B8234D492788083735B`

```text
GEO_WHEEL_FL_TIRE_INNER.png
GEO_WHEEL_FR_TIRE_INNER.png
GEO_WHEEL_RL_TIRE_INNER.png
GEO_WHEEL_RR_TIRE_INNER.png
GEO_WHEEL_TIRE_INNER.png
```

### Group D — wheel tread, five copies

SHA-256: `9D2DDFF867198D1DC57C9BCBD4FF941FE074F1CCA2BEE14E303484D7D12A3484`

```text
GEO_WHEEL_FL_TREAD.png
GEO_WHEEL_FR_TREAD.png
GEO_WHEEL_RL_TREAD.png
GEO_WHEEL_RR_TREAD.png
GEO_WHEEL_TREAD.png
```

### Group E — wheel outer faces, five copies

SHA-256: `DE7DD08D2E565F13F328DAD1A2D25E0FF15DC08435381172F726B0DBFFCE4A63`

```text
GEO_WHEEL_FL_TIRE_OUTER.png
GEO_WHEEL_FR_TIRE_OUTER.png
GEO_WHEEL_RL_TIRE_OUTER.png
GEO_WHEEL_RR_TIRE_OUTER.png
GEO_WHEEL_TIRE_OUTER.png
```

## 5. Existing report inventory

| File | Bytes | SHA-256 |
|---|---:|---|
| `body_refinement_report.json` | 4019 | `996E843FDFE170320982BF82FDDEFCBD9E0F10F65F8458E113CBE689C7857D1C` |
| `manifest.json` | 14077 | `AFB6EAF3B6B53F05E1F6F9C40CADA7DF8314707C42D385F8F5F12AEB882B333F` |
| `rear_refinement_report.json` | 3174 | `2E571EB8EA2A40E81A8AEA3688D074872B67752361FA152876AEBFF5995406D0` |
| `sanitation_report.json` | 1355 | `54F1C3502C67394384415EDA0581476D330DAB3F1EDBE8A692FBC687A3844CEF` |
| `sidepods_refinement_report.json` | 4778 | `EF50EFB3E3DC2552810C236A763C978545FA1A9AE393BB2CED840B270CE32260` |
| `wheel_anchor_map.json` | 2103 | `820D44CCBBA6C13D536CBEEDB3FB5D4606A05F966B8F1899AD2409195DD2D9A2` |
| `wheel_refinement_report.json` | 24574 | `84541C57976A0DD5FF817B77CBC57E044A1667D4B6EE9E7634EFC0693188D686` |
| `wheel_retexture_report.json` | 830 | `22CD95A388EF7A6159851BDCBFAF0BE039A17D02952C4B0DDCED29B150CC1DD9` |
| `wheel_uv_remap_report.json` | 9196 | `9B86919DDE459D3228D51B377F225D441AD7EB74D99B1353DECA461AEEC2302C` |
| `wing_split_report.json` | 27092 | `41445E1B4E3E23B1F7C4EAD906FA05401F752B42D0247409869AE48A1491A1CC` |

## 6. Baseline acceptance

- Source mutation: `0` files.
- Covered source files: `55`.
- Structural GLB probe repetition: `PASS`.
- Geometry, UV, normal and semantic-node limitations: recorded.
- Visual acceptance: `NOT_OBSERVED`; not required for this read-only item.

VS-001 result: `REVIEWED`.

