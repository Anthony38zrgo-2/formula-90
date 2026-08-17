# Formula-90 Agent Protocol

AUTHORITY: `.agents/AGENTS.md`
INVARIANT: Fail Faster, Adapt Faster — cheapest falsification before mutation.

PIPELINE:
classify -> preflight -> baseline? -> ownership -> hypothesis -> cheapest experiment -> 1 reversible delta -> validate -> keep|rollback

GUARDRAILS:
CTX-FIRST; NO-BROAD-DISCOVERY; SUBAGENT=EXECUTOR; 1-HYPOTHESIS/ITER; REUSE-BUILDS; STOP-ON-MISSING-CONTEXT; PRESERVE-WORKTREE; ARCH-SCOPE-LOCK.

ARCH-SCOPE-LOCK: gameplay architecture changes only if explicitly requested or causally proven by evidence.
