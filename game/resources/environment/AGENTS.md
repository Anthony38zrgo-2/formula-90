# environment — Procedural environment pipeline

Scope: the trackside environment pipeline: source media, manifests, palettes, recipes, and pipeline tests.
Consumers: circuit dressing in track scenes.
Rules: manifests declare what a build contains; recipes declare how. The `generated/` build output stays untracked without its own file; media leaves without manifests (`examples/`, media under `assets/`) carry no AGENTS.md and are covered here.
Subfolders: `assets/`, `manifests/`, `palettes/`, `recipes/`, and `tests/` document themselves in their own AGENTS.md.
