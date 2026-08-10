---
name: knowledge-query
description: Query the local curated knowledge database before guessing framework, API, version, or project-specific technical facts.
---

# Knowledge Query

Use the Rust `agentdb` runtime. Do not open whole manuals unless indexed retrieval misses.

1. Identify the channel: `godot`, `blender`, `cpp`, or `python`.
2. Query exact symbols/terms first: `agentdb knowledge <channel> <term>`.
3. Apply only results compatible with the current project/tool version.
4. If the result conflicts with verified repository/runtime evidence, repository/runtime evidence wins and the knowledge entry should be reviewed.
5. If no result exists, escalate to the canonical manual/research path rather than inventing an API.
