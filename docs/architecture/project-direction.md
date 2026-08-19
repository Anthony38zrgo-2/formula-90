# Formula90s — Project Direction

This document defines the target state of Formula90s. It is intended to be durable context for humans and coding agents so that future changes converge on the same game instead of optimizing isolated subsystems in incompatible directions.

## 1. Product goal

Formula90s is a deliberately compact 1990s-inspired formula racing game. The target is not a modern full-simulation package and not a generic arcade racer. The goal is a convincing, readable, mechanically expressive racing game whose handling, presentation and production pipeline all feel coherent with late-1990s console racing games while retaining deterministic modern tooling underneath.

The project should favor:

- progressive, understandable vehicle behavior;
- real circuit identity without survey-grade reconstruction requirements;
- low-poly 3D geometry and strong texture work rather than modern asset density;
- short iteration loops backed by telemetry and deterministic generators;
- reusable systems that allow AI-assisted development without turning the repository into a collection of one-off generated artifacts.

The current development vehicle is the Jordan 1995 configuration and the current handling-development circuit is La Chutana.

## 2. Artistic direction

### 2.1 Reference character

The environment should evoke late-1990s PS1 rally/racing presentation:

```text
PS1 rally/racing scenic aesthetic
+ strong readable silhouettes
+ low geometry density
+ painted/prerendered-looking textures
+ fake/baked shadow information
+ restrained color count
+ purposeful roadside composition
+ macro terrain variation
```

The objective is not to imitate rendering defects blindly. The target is a clean interpretation of the period: visibly retro, inexpensive to render, but compositionally deliberate.

### 2.2 Environment hierarchy

Scenery exists to support driving, orientation, speed perception and circuit identity.

Priority order:

1. road geometry and surface readability;
2. curbs, edges, runoff and safety infrastructure;
3. nearby vegetation and landmarks that communicate speed;
4. medium-distance tree masses, structures and terrain forms;
5. distant skyline/background support.

Generic scenery should not be modeled in full 3D merely because modern racing games do so.

A practical rule is:

> If an object does not affect driving, orientation, speed sensation or circuit identity, it probably does not need detailed 3D geometry.

### 2.3 Track surfaces

The road must visually belong to the terrain instead of appearing as an isolated ribbon on a flat plane.

Desired transition:

```text
asphalt
→ painted edge / curb
→ immediate dirt or dry shoulder
→ mixed dry/green terrain
→ grass cards / bushes
→ trees / structures / horizon
```

Terrain pigmentation should use broad asymmetric areas instead of uniform random noise. A circuit can contain greener zones, dry grass, exposed earth and localized wear while remaining visually coherent.

### 2.4 Vegetation contract

Vegetation is intentionally cheap geometry whose texture carries most of the visual information.

- Trees: three crossed textured planes, giving six directional faces.
- Bushes: two crossed planes, wider and lower than trees.
- Grass: one double-sided card per instance.
- Vegetation cards do not use gameplay collision.
- Texture silhouettes should include fake/prerendered shading and enough internal contrast to imply volume.
- Tree and bush palettes must be distinct enough that the environment does not collapse into a single green mass.

The biome model is artistic rather than scientifically exact:

```text
continent
+ longitudinal band: west | center | east
+ altitude band: low | medium | high
```

South America currently provides all nine combinations. La Chutana uses:

```text
continent = south_america
longitude = west
altitude = low
```

This represents a dry/semi-arid Peruvian coastal direction with a balanced mixture of green, dry grass and exposed soil.

### 2.5 Structures

Buildings are secondary scenery. Use only enough medium/far structures to establish place and depth.

Current rules:

- simple low-poly 3D shells;
- four walls plus a minimal top;
- no unnecessary underside;
- mixed residential and industrial visual language for La Chutana;
- dimensions may vary procedurally, but the base grammar remains simple;
- structures should not dominate a circuit unless the real location requires them for identity.

### 2.6 Texture Forge

The artistic source image is not the final runtime texture. Formula90s uses a deterministic texture-processing stage so AI-generated or procedural bases converge toward one visual language.

The intended pipeline is:

```text
source / generated base
        ↓
Formula90s Texture Forge
        ↓
alpha cleanup / silhouette simplification
palette control
fake directional lighting
fake ambient occlusion
lower/contact shadow
posterization
subtle ordered dithering
hash + recipe manifest
        ↓
Blender material
        ↓
Godot runtime
```

Same source + recipe + biome + seed must produce the same pixels.

The current style identifier is:

```text
ps1_rally_clean
```

Agents should modify explicit recipes and configuration rather than manually introducing random one-off image edits.

## 3. Programming direction

### 3.1 Core architecture

Formula90s should remain split by responsibility.

```text
Godot
├── gameplay runtime
├── GEVP vehicle physics
├── Formula90s controller / assists
├── HUD / camera / telemetry
└── generated track consumption

Python
├── deterministic track reconstruction
├── validation
├── environment placement
├── texture generation / Texture Forge
└── reproducible data transforms

Blender Python
├── final track mesh construction
├── curbs
├── terrain
├── guardrails
├── procedural scenery geometry
└── GLB export
```

Python is the deterministic build layer. Blender is the scene/mesh assembler. Godot is the runtime authority.

### 3.2 Vendor isolation

GEVP vendor code must not be casually modified to solve Formula90s-specific problems.

Prefer:

- Formula90s subclasses;
- configuration;
- adapters;
- scene composition;
- explicit surface groups;
- project-owned scripts.

A visual, track or content problem must not be hidden by retuning global vendor physics.

### 3.3 Determinism as a design requirement

AI-assisted development creates a risk of generating different solutions each time a problem is revisited. Formula90s counters this by making procedural systems deterministic.

For generators:

```text
configuration
+ source data
+ seed
+ pipeline version
= reproducible output
```

Required practices:

- stable seeds;
- explicit numeric configuration;
- versioned recipes;
- validation before publish;
- output manifests and hashes where practical;
- no hidden random state;
- no agent-only knowledge required to reproduce an asset.

The AI agent may act as planner or art director, but Python should own the repeatable transformation from specification to artifact.

### 3.4 Generated asset authority

Generated source and runtime outputs are distinct concepts.

The track workflow is intentionally gated:

```text
reference data
→ prepare_track.py
→ validate_track.py
→ Texture Forge
→ Blender Base build
→ human gameplay validation
→ procedural placement
→ environment validation
→ Blender Procedural build
→ canonical GLB
```

`track_base.blend` is the clean validated source for a procedural run. A procedural run must reopen the clean Base rather than decorate the previous procedural output.

Previous `.blend` files are backed up. GLB publication must be atomic so a failed generation does not delete the last known-good runtime track.

### 3.5 Collision is independent from visual complexity

Visual geometry should not automatically become collision geometry.

Examples:

- guardrails use simple collision proxies rather than detailed corrugated visual meshes;
- vegetation cards have no gameplay collision;
- terrain collision is explicitly generated and validated;
- visual terrain may have a small offset or sink that collision terrain does not share;
- safety geometry may exist invisibly when it prevents catastrophic world escape without affecting normal driving.

Collision correctness has higher priority than visual fidelity.

## 4. Vehicle handling objective

The target is a convincing arcade/simulation hybrid inspired by late-1990s formula games: progressive steering, readable weight transfer and recoverable loss of grip rather than instant digital adhesion.

Desired driver experience:

- steering builds rather than snapping immediately to maximum lateral grip;
- oversteer can be countersteered if corrected in time;
- a late or excessive correction can still produce a spin;
- lifting the throttle can help recover grip;
- braking and weight transfer remain understandable;
- off-track surfaces change behavior without acting like invisible walls;
- aerodynamic balance later changes high-speed behavior without masking mechanical handling problems.

## 5. Vehicle-development phases

Only one major behavioral family should be tuned at a time.

### Phase A — geometry and physical layout — complete

Purpose:

- correct wheelbase/track geometry;
- correct wheel RayCast positions;
- stable chassis collision;
- correct visual wheel contract;
- Jordan dimensions without changing the validated baseline behavior unnecessarily.

### Phase B — mechanical grip / brakes / differential — complete baseline

Purpose:

- road/curb/gravel/grass behavior;
- mechanical grip differentiation;
- braking characteristics;
- differential baseline;
- robust wheel/landing behavior.

The current Phase B state is the reference that must survive track-generation work.

### Track-validation gate — current work

Before Phase C, La Chutana must be stable enough to expose handling problems rather than introduce track-collision artifacts.

Required before moving on:

- full lap without hidden collision discontinuities;
- reliable road→grass and grass→road transitions;
- safe curb contact at multiple speeds;
- no infinite fall/noclip state;
- correct Road/Curb/Grass surface classification;
- reproducible Base generation.

### Phase C — steering and countersteer

Only steering behavior should change here.

Targets:

- progressive keyboard steering;
- controllable initial response;
- natural unwind;
- useful but non-omniscient countersteer assistance;
- recoverable oversteer while retaining a real spin threshold.

Do not simultaneously retune tires, suspension, powertrain or aero.

### Phase D — 1995 suspension behavior

Tune:

- springs;
- damping;
- anti-roll behavior if used;
- transient weight transfer;
- braking pitch and direction changes;
- curb/bump response.

Do not use extra tire grip to conceal suspension problems.

### Phase E — V10 powertrain and transmission

Replace reference powertrain behavior with the intended 1990s formula character:

- torque delivery;
- RPM range;
- engine braking;
- throttle response;
- gear ratios;
- interaction with differential.

### Phase F — aerodynamics

Add and tune:

- front load;
- rear load;
- aero balance;
- drag;
- speed-dependent behavior.

Mechanical handling must already work before aero is used to shape high-speed balance.

### Phase G — assists and true no-assists behavior

Separate clearly:

```text
natural vehicle physics
GEVP baseline stabilization
Formula90s optional driving aids
```

`OFF` should eventually have a precise behavioral meaning rather than merely falling back to an undocumented layer of stabilization.

## 6. Track-development objective

Real circuits should be reconstructed deterministically from references and physical anchors rather than eyeballed repeatedly by an agent.

Authority order should generally be:

1. geospatial/vector data when available;
2. georeferenced imagery;
3. orthographic reference diagrams;
4. corrected perspective imagery;
5. manual approximation only as fallback.

A circuit is successful when its geometry is consistent, recognizable and useful for gameplay. It does not need survey-grade CAD precision unless a future requirement explicitly demands it.

Curbs must use smooth low profiles appropriate for a low formula chassis. Large rectangular curb blocks that launch the car are forbidden.

## 7. AI-driven development rules

Formula90s is intentionally compatible with multi-agent development, but agent autonomy must be constrained by reproducible contracts.

Agents should:

- read repository documentation before modifying a subsystem;
- inspect current code instead of assuming architecture;
- preserve frozen/validated baselines;
- use telemetry before changing physics;
- distinguish geometry bugs from vehicle bugs;
- prefer configuration and generators over manual duplicated assets;
- validate before publishing generated content;
- make commits that state what changed, not what was hoped to change.

Agents should not:

- brute-force a recurring issue by repeatedly changing unrelated parameters;
- retune physics to hide broken track collision;
- modify vendor code for project-specific presentation problems;
- generate persistent asset variants solely for different scale or color;
- overwrite a known-good generated source without backup;
- delete the runtime asset before a replacement has completed successfully;
- claim a generated track is validated before it has been driven by a human.

## 8. Performance philosophy

Visual richness should come from reuse and composition rather than raw asset count.

Preferred techniques:

- shared meshes;
- texture atlases where useful;
- small textures;
- cards and crossed planes;
- per-instance transform/tint variation;
- MultiMesh/instancing in Godot when the runtime integration is mature;
- simple collision proxies;
- distant scenery represented cheaply.

The final game should remain inexpensive to package and render despite procedural variety.

## 9. Definition of project coherence

A change is aligned with Formula90s when it improves one or more of the following without damaging the others:

```text
handling readability
track reliability
retro visual identity
deterministic reproducibility
iteration speed
runtime efficiency
```

When choosing between a modern complicated solution and a simple deterministic solution that produces the desired late-1990s racing character, prefer the latter.
