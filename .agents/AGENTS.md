# Formula-90 Agent Protocol

AUTHORITY: `.agents/AGENTS.md`
INVARIANT: Fail Faster, Adapt Faster — cheapest falsification before mutation.

PIPELINE:
classify -> preflight -> baseline? -> ownership -> hypothesis -> cheapest experiment -> 1 reversible delta -> validate -> keep|rollback

GUARDRAILS:
SCOPE-LOCK; CTX-FIRST; ACTIVE-SKILLS-ONLY; NO-LIB-SCAN; NO-BROAD-DISCOVERY; NO-SMOKE; BUILD!=BEHAVIOR; DIAG-FIRST; RUNTIME-FIRST; FACT!=INFERENCE; NO-WEAKEN; HUMAN-GATE; SUBAGENT=EXECUTOR; OWNER-EXPLICIT; REVIEW=RO; 1-HYPOTHESIS/ITER; REUSE-BUILDS; STOP-ON-MISSING-CONTEXT; PRESERVE-WORKTREE; ARCH-SCOPE-LOCK.

SKILLS:
LOAD=`.agents/skills/*`; LIBRARY=OFF unless an active skill explicitly references it; MAX-ACTIVE=3.

EVIDENCE:
CRASH/EXIT > STRUCTURED-EVENTS > TELEMETRY/DELTA > BUILD > DIRECT-REGRESSION > STATIC.
NO-ERROR != CORRECT; UNOBSERVED => `NOT_OBSERVED`.

GATE:
AGENT-MAX=`READY_FOR_HUMAN_GATE`; HUMAN-ONLY=`ACCEPTED|REJECTED`; `diag.py gate` only on explicit human decision; second `REJECTED` => STOP.

SUBAGENTS:
SPAWN only for independent evidence; NON-OVERLAP ownership; EXECUTOR=write-scope only; REVIEWER=RO; NO-REPLAN.

ARCH-SCOPE-LOCK: gameplay architecture changes only if explicitly requested or causally proven by evidence.

SOT:
RUNTIME=`diagnostics/runtime.sqlite`; AGENT-KB=`.agents/data/agents.sqlite`.
TOOLS: `diag.py`=query evidence; `run_godot.py`=real-runtime capture.
