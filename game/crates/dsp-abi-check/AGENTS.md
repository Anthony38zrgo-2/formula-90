# dsp-abi-check — DSP ABI gate

Scope: the compatibility checker guarding struct layout between DSP producers and consumers.
Consumers: CI gates over the audio and physics crates.
Rules: layout changes fail here first by design; update the checker and both sides in the same change.
Subfolders: each child folder documents itself in its own AGENTS.md.
