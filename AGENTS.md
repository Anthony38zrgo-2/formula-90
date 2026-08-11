# Formula-90 agent workflow

The operational guide is [`.agents/AGENTS.md`](.agents/AGENTS.md).

Default iteration is simple:

```text
hypothesis -> smallest reversible change -> targeted test -> keep or revert
```

Use heavier diagnostics only for repeated uncertainty, unclear ownership,
cross-system failures, milestones, or high-risk changes. Keep gameplay
architecture unchanged unless the task explicitly changes it.

Before editing a Godot `.tscn`, use `scene-safety`. Before vehicle tuning, use
`vehicle-physics`; use telemetry and full regression only when they answer a
real uncertainty or a milestone/high-risk need.

Clear tasks may be completed directly. Delegate complex work only when useful;
the compact handoff format is defined in `.agents/AGENTS.md`.
