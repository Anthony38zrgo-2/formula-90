# Runtime Diagnostics

SCOPE: startup/runtime regressions and runtime-sensitive changes.
SOT: `diagnostics/runtime.sqlite` + raw logs.
POLICY: NO-SMOKE; DIAG-FIRST; HYPOTHESIS-BEFORE-MUTATION.

PRE:
```bash
python .agents/tools/diag.py recent --limit 5
python .agents/tools/diag.py crashes --limit 10
python .agents/tools/diag.py errors --limit 20
```

POST:
BUILD(real-target) -> REAL-RUNTIME -> RUN-ID -> SUMMARY -> CRASHES -> ERROR/WARN -> MIN-TELEMETRY -> accepted-baseline?

QUERY:
```bash
python .agents/tools/diag.py summary RUN_ID
python .agents/tools/diag.py crashes --run-id RUN_ID
python .agents/tools/diag.py errors --run-id RUN_ID
python .agents/tools/diag.py telemetry RUN_ID --from-frame 0 --limit 200
python .agents/tools/diag.py baseline --task "TASK NAME"
```

STATES:
`BUILT|RUNTIME_STARTED|GAME_READY|RUNTIME_OBSERVED|RUNTIME_ERROR|CRASHED|NOT_OBSERVED|READY_FOR_HUMAN_GATE`.
`ACCEPTED` = human-recorded only.
