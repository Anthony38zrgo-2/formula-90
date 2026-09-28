# src — Synthesis implementation

Scope: synthesis library, stage models, and runner binaries.
Consumers: offline bank authoring.
Rules: stage interfaces stay stable; experiments land behind the existing stage seams. The `bin/` synthesis-runner folder is ignore-blocked from carrying its own file and is covered here: outputs default to `scratch/`, promotion follows the output policy.
