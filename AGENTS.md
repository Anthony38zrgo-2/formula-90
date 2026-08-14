# Formula-90 agent workflow

The operational authority is [`.agents/AGENTS.md`](.agents/AGENTS.md).

The canonical rule is **Fail Faster, Adapt Faster**:

> Never make a code change when a cheaper experiment can reject the hypothesis first.

Use this order: classify -> preflight -> baseline when behavior matters -> identify
ownership -> form a falsifiable hypothesis -> run the cheapest useful experiment ->
make one reversible causal change -> validate immediately -> keep or rollback.

Fail Fast does not mean implement immediately. It means invalidate wrong assumptions
early. Adapt Fast means update the problem model from evidence, not switch randomly.

Before editing a Godot `.tscn`, use `scene-safety`. Before vehicle tuning, use
`vehicle-physics`. Use the relevant domain skill for aero, powertrain, telemetry,
track generation, or physics diagnostics.
For 3D asset generation, analysis, modification, or export, use
`3d-asset-generation` and prefer its deterministic Python tooling before Blender.

For 3D vehicles, prove asset-forward versus runtime-forward from geometry and
RayCast datums before editing physics. Godot import does not by itself prove a
semantic `+Z` to `-Z` conversion. Smokes must validate forward alignment, axle
mapping and front/rear wheel resources, not only node/material presence.

Before running Godot headless, ensure `user://` is writable. A crash before test
assertions, especially `Failed to open user://logs`, is `INCONCLUSIVE` and must
not trigger a production patch. Never treat a failed file/hash read as a content
mismatch.

Stop implementation and enter Diagnostic Mode when the same failure signature
survives two implementation attempts, ownership is unknown, validation is broken,
or the next patch would stack a workaround. Diagnostic Mode permits inspection,
telemetry, parsers, offline models, minimal reproductions, and tests, but no
production patches.

Preserve unrelated work. Keep gameplay architecture unchanged unless the task
explicitly changes it or verified evidence identifies it as the cause.

## Git shell

All repository Git operations (`status`, `diff`, `add`, `commit`, `fetch`,
`pull`, `push`, branch and remote operations) must run through Git Bash. On
Windows, invoke `C:\Program Files\Git\bin\bash.exe`; do not use PowerShell's
Git invocation for repository operations.

## Agent escalation gate

Never promote, escalate, hand off, delegate, or transfer any task to another
agent, subagent, model, or a higher-capability model unless the user explicitly
requests that specific action in the current task. This prohibition applies in
every context, including planning, implementation, diagnostics, validation,
reviews, retries, time pressure, or suspected task complexity. A prior approval
for delegation, a prior model choice, completion of local attempts, or the
availability of a stronger model does not constitute approval for a new
escalation.
