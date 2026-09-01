# Current Rust V10 audio architecture

This diagram describes the implemented Rust-only signal path as of the
`GF460` 7,499 RPM validation render. C++ and Faust are not part of this path.

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

        BLOCK --> GEARBOX["Gearbox housing / transmission bell<br/>246–588 Hz reinforced casing modes<br/>attenuated gear-mesh family<br/>0.38 ms structural arrival"]
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

        BLOCK --> MOUNTS["Engine mounts → monocoque<br/>148–548 Hz damped structural modes<br/>0.18 / 0.82 ms transmissions"]
        COLLECTORS --> MOUNTS
        MOUNTS --> SEAT["Under-seat vibration<br/>92–263 Hz body-coupled modes<br/>1.45 ms additional transmission"]

        AIR --> COCKPIT["Cockpit cavity<br/>168–742 Hz broad modes<br/>2.15–7.90 ms early reflections<br/>1.6 kHz absorption"]
        AIRBOX --> COCKPIT
        COVER --> COCKPIT
        REAR --> COCKPIT

        BULKHEAD --> LOWMID["Low-mid parallel compression<br/>150–560 Hz · 2.5:1<br/>6 ms attack · 52 ms release"]
        GEARBOX --> LOWMID
        MOUNTS --> LOWMID
        SEAT --> LOWMID
        COCKPIT --> LOWMID
        LOWMID --> SAT["Load-dependent structural saturation<br/>105–680 Hz pre-band<br/>asymmetric drive · 920 Hz post-filter"]

        AIR --> MIX["Cockpit capture mix"]
        BULKHEAD --> MIX
        GEARBOX --> MIX
        COVERS --> MIX
        AIRBOX --> MIX
        COVER --> MIX
        REAR --> MIX
        MOUNTS --> MIX
        SEAT --> MIX
        COCKPIT --> MIX
        LOWMID --> MIX
        SAT --> MIX
    end

    MIX --> OUTPUT["Scene output<br/>GF460 reference render"]

    DRY -. diagnostic .-> STEMS["Auditable stems"]
    AIR -. diagnostic .-> STEMS
    BULKHEAD -. diagnostic .-> STEMS
    GEARBOX -. diagnostic .-> STEMS
    COVERS -. "A · B · combined" .-> STEMS
    AIRBOX -. diagnostic .-> STEMS
    COVER -. diagnostic .-> STEMS
    REAR -. "A · B · combined" .-> STEMS
    MOUNTS -. "mounts · monocoque · combined" .-> STEMS
    SEAT -. diagnostic .-> STEMS
    COCKPIT -. diagnostic .-> STEMS
    LOWMID -. diagnostic .-> STEMS
    SAT -. diagnostic .-> STEMS
    MIX -. diagnostic .-> STEMS

    classDef source fill:#5b2333,color:#fff,stroke:#d78a9c,stroke-width:2px;
    classDef physical fill:#263859,color:#fff,stroke:#79a7d3,stroke-width:2px;
    classDef capture fill:#3b5d3b,color:#fff,stroke:#9ac39a,stroke-width:2px;
    classDef output fill:#6b5428,color:#fff,stroke:#e7c56d,stroke-width:2px;
    class INPUT,CRANK,CYL source;
    class PRESSURE,HEADERS,COLLECTORS,TURB,BLOCK,DRY physical;
    class AIR,BULKHEAD,GEARBOX,COVERS,AIRBOX,COVER,REAR,MOUNTS,SEAT,COCKPIT,LOWMID,SAT capture;
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
| Engine mounts/monocoque | Damped torque and block transmission in low mids |
| Under-seat vibration | Body-coupled weight below 300 Hz |
| Cockpit cavity | Absorbed low-mid air volume and early reflections |
| Low-mid parallel | Sustained 150–560 Hz structural body without master pumping |
| Load saturation | Torque-dependent low-mid density with asymmetric coloration |

The renderer writes each path independently so perceptual changes can be
validated without guessing which subsystem caused them.
