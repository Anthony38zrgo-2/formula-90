# Formula-90 `.agents`

PURPOSE: minimal-context agent harness; NO-SMOKE; OBSERVABILITY-FIRST; HUMAN-GATE; PROFILED-SKILLS.

BOOTSTRAP:
```bash
python .agents/tools/init.py
python .agents/tools/profile.py list
python .agents/tools/profile.py activate minimal
```

PROFILES:
```bash
python .agents/tools/profile.py activate rust-physics
python .agents/tools/profile.py activate godot-runtime
```

DIAG:
```bash
python .agents/tools/diag.py recent --limit 5
python .agents/tools/diag.py crashes --limit 10
python .agents/tools/diag.py errors --limit 20
```

REAL-RUNTIME:
```bash
python .agents/tools/run_godot.py --project . --task "describe the task" --profile godot-runtime
```

READY-MARKER: exact `[OBSERVABILITY] GAME_READY` => `game_ready=1`.

LAYOUT:
- `AGENTS.md` = always-on protocol
- `library/skills/` = canonical/off-context
- `skills/` = active/generated only; MAX=3
- `profiles/` = skill sets
- `workflows/` = orchestration
- `data/agents.sqlite` = agent KB
- `diagnostics/runtime.sqlite` = runtime SOT
- `diagnostics/raw/` = raw logs
- `diagnostics/exports/` = CSV

PROFILE-ACTIVATION: copy canonical skill -> active skill; canonical library remains immutable.
