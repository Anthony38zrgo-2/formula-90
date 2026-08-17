# Vehicle Physics

SCOPE: dynamics; tires; suspension; steering; brakes; aero; drivetrain; Rust/Godot physics coupling.

FLOW:
ownership -> regression? DIAG-FIRST -> inspect model+units -> define observables -> smallest delta -> BUILD -> REAL-RUNTIME -> before/after delta -> HUMAN-GATE(feel).

POLICY:
EXISTING-MODEL>PREFER; NO-GENERIC-REPLACEMENT without evidence; MEASURE-BEFORE-INFER; subjective handling != numeric acceptance.

REF-ON-DEMAND:
suspension/dampers/load-transfer=`references/suspension.md`
tire slip/grip/forces=`references/tires.md`
RPM/gearing/torque=`references/powertrain.md`
drag/downforce/balance=`references/aero.md`
UNRELATED-REF=OFF.
