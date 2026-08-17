# Feature Loop

RULE: smallest valid loop; PROCESS-CEREMONY=OFF.

FAST:
EXEC -> BUILD(real-target) -> RUNTIME? -> DIAG -> `READY_FOR_HUMAN_GATE` -> HUMAN-GATE.

FEATURE:
PLAN-A=`Hy3 High` -> EXEC-A=`DeepSeek V4 Flash Max` -> BUILD+REAL-RUNTIME+DIAG -> REVIEW-A=`Hy3 High`(clean,RO) -> HUMAN-GATE.

PLAN-A:
SCOPE+EVIDENCE -> subagents? only independent/RO -> ACCEPTANCE-CRITERIA before mutation.

EXEC-A:
FOLLOW-PLAN; NO-AC-DRIFT; NO-SMOKE.

REVIEW-A:
REQUIREMENTS + DIFF + RUNTIME-EVIDENCE + DIAG; executor tests=UNTRUSTED until coverage inspection; REVIEW-SUBAGENTS=RO.

ESCALATE:
trigger=first `REJECTED`.
FAILURE-PACKAGE=requirement+diff+diag+telemetry+review+human-feedback.
PLAN-B=`GPT Luna Max`; ASSUME PLAN-A MAY-BE-WRONG; ROOT-CAUSE={requirement|architecture|implementation|integration|asset|engine/runtime|measurement}.
EXEC-B=`Hy3 High` -> REAL-RUNTIME+DIAG -> HUMAN-GATE.
second `REJECTED` => STOP/AWAIT-HUMAN.

SUBAGENTS:
GOOD=independent repo evidence|independent subsystem diagnosis|adversarial RO review.
BAD=overlapping writers|generic repo exploration|capability-driven spawning.
