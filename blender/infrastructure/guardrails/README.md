# Procedural guardrails

The default racetrack pipeline generates its own low-poly classic guardrail modules. No source `.blend`/`.glb` is required.

Configuration controls:

- module length;
- W-beam-like visual height/depth;
- post spacing and dimensions;
- circuit fractions/side;
- distance from track edge;
- simplified collision height/thickness.

The visible rail is assembled from a few shared low-poly strips and posts. Collision is always a separate box proxy named with Godot's `-colonly` suffix so the detailed visual mesh is never used for vehicle collision.

This folder remains available for a future hand-authored visual override, but procedural generation is the default and must continue to work with the folder empty.
