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

Stop implementation and enter Diagnostic Mode when the same failure signature
survives two implementation attempts, ownership is unknown, validation is broken,
or the next patch would stack a workaround. Diagnostic Mode permits inspection,
telemetry, parsers, offline models, minimal reproductions, and tests, but no
production patches.

Preserve unrelated work. Keep gameplay architecture unchanged unless the task
explicitly changes it or verified evidence identifies it as the cause.
