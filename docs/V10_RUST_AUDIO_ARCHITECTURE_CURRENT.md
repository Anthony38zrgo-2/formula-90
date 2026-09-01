# Current Rust V10 audio architecture

This diagram describes the implemented Rust-only signal path as of the
`GF363` 7,499 RPM validation render. C++ and Faust are not part of this path.

```mermaid
flowchart LR
    INPUT["EngineInput<br/>RPM · throttle · load"] --> CRANK["Crankshaft<br/>720° cycle · 10 firing events"]
    CRANK --> CYL["10 explicit cylinders<br/>combustion pressure · valve lift · blowdown"]

    CYL --> PRESSURE["Pressure paths<br/>direct · derivative A/B"]
    CYL --> HEADERS["10 individual headers<br/>geometry delay · reflection"]
    HEADERS --> COLLECTORS["Collectors A/B<br/>chamber pressure · radiation"]
    CYL --> TURB["Event-gated turbulence<br/>460–2050 Hz"]

    PRESSURE --> BLOCK["BlockHead<br/>crankcase · block · head modes"]
    BLOCK --> DRY["V10Engine dry master"]
    COLLECTORS --> DRY
    TURB --> DRY

    subgraph SCENE["AcousticScene — RearBulkheadCockpit"]
        DRY --> AIR["Remote engine-air path<br/>3-band attenuation · mid ducking<br/>1.6 / 3.4 / 5.8 ms arrivals"]

        BLOCK --> BULKHEAD["Rear bulkhead / metallic structure<br/>heavy castings · bellhousing family<br/>thin panels · structural coloration"]
        PRESSURE --> BULKHEAD
        COLLECTORS --> BULKHEAD

        BLOCK --> GEARBOX["Gearbox housing / transmission bell<br/>246–1147 Hz casing modes<br/>1.4–3.2 kHz gear-mesh modes<br/>0.38 ms structural arrival"]
        COLLECTORS --> GEARBOX

        PRESSURE --> COVERS["Cylinder-head covers A/B<br/>independent bank excitation<br/>1.18–5.29 kHz panel modes<br/>0.72 / 0.96 ms arrivals"]
        HEADERS --> COVERS
        TURB --> COVERS

        PRESSURE --> AIRBOX["Airbox / intake plenum<br/>176–471 Hz Helmholtz cavity<br/>612–2083 Hz runner modes<br/>2.6 ms airborne arrival"]
        HEADERS --> AIRBOX
        TURB --> AIRBOX

        BLOCK --> COVER["Engine cover / bodywork skin<br/>684–1982 Hz broad panels<br/>2.4–5.1 kHz upper skin<br/>1.35 ms coupled arrival"]
        AIRBOX --> COVER
        TURB --> COVER

        COLLECTORS --> REAR["Rear exhaust capture A/B<br/>584 Hz–4.45 kHz pipe modes<br/>load-dependent radiation<br/>1.85 / 2.20 ms arrivals"]
        HEADERS --> REAR

        AIR --> MIX["Cockpit capture mix"]
        BULKHEAD --> MIX
        GEARBOX --> MIX
        COVERS --> MIX
        AIRBOX --> MIX
        COVER --> MIX
        REAR --> MIX
    end

    MIX --> OUTPUT["Scene output<br/>GF363 reference render"]

    DRY -. diagnostic .-> STEMS["Auditable stems"]
    AIR -. diagnostic .-> STEMS
    BULKHEAD -. diagnostic .-> STEMS
    GEARBOX -. diagnostic .-> STEMS
    COVERS -. "A · B · combined" .-> STEMS
    AIRBOX -. diagnostic .-> STEMS
    COVER -. diagnostic .-> STEMS
    REAR -. "A · B · combined" .-> STEMS
    MIX -. diagnostic .-> STEMS

    classDef source fill:#5b2333,color:#fff,stroke:#d78a9c,stroke-width:2px;
    classDef physical fill:#263859,color:#fff,stroke:#79a7d3,stroke-width:2px;
    classDef capture fill:#3b5d3b,color:#fff,stroke:#9ac39a,stroke-width:2px;
    classDef output fill:#6b5428,color:#fff,stroke:#e7c56d,stroke-width:2px;
    class INPUT,CRANK,CYL source;
    class PRESSURE,HEADERS,COLLECTORS,TURB,BLOCK,DRY physical;
    class AIR,BULKHEAD,GEARBOX,COVERS,AIRBOX,COVER,REAR capture;
    class MIX,OUTPUT,STEMS output;
```

## Current mix intent

The virtual listening position is fixed to the rear cockpit bulkhead behind the
driver. Structural paths arrive before the attenuated airborne engine path.
The full-range dry master is retained only as a diagnostic stem; it is not mixed
directly into the cockpit output.

| Capture path | Implemented role |
|---|---|
| Remote engine air | Filtered mass and restrained direct engine identity |
| Rear bulkhead | Main medium-frequency structural resonance |
| Gearbox housing | Lower, heavier casting and torsional resonance |
| Head covers A/B | Short, bright mechanical detail from each bank |
| Airbox/plenum | Airborne intake body and aspiration |
| Engine cover | Hollow, laminated bodywork-panel radiation |
| Rear exhaust A/B | Load-dependent collector and tailpipe radiation |

The renderer writes each path independently so perceptual changes can be
validated without guessing which subsystem caused them.
