# Formula-90 Context Protocol Manifest

## CORE

```text
STATE = current-state
CTX   = context-guide
ITEM  = <AREA>-<NNN>
FLOW  = STATE(scope,pick) -> CTX(item) -> EXEC
DISCOVERY = CTX-MISS only
```

## AREA

```text
AE   = Aerodynamics
PHY  = Vehicle Physics
TYR  = Tires
PWR  = Powertrain
SUS  = Suspension
BRK  = Brakes
HUD  = HUD/UI
AUD  = Audio
TRK  = Track
AST  = Assets
SYS  = Architecture/Infrastructure
TOOL = Tooling/Agents
```

## PICK

```text
TOP     = highest priority actionable
READY   = highest priority unblocked
ACTIVE  = current work item
LATEST  = most recently added
BLOCKED = highest priority blocked
ALL     = compact area backlog
```

## STATE SYNTAX

```text
/state AE:READY
/state SUS:TOP
/state PWR:ACTIVE
/state AE:LATEST
/state AE-014
/state AE-014 DETAIL
```

Equivalent normalized form:

```text
STATE AREA=AE PICK=READY
STATE ID=AE-014
STATE ID=AE-014 DETAIL
```

## CTX SYNTAX

```text
/ctx AE-014
/ctx AREA=SUS SUBJECT=preload
/ctx SYMBOL=AeroModel
/ctx PATH=game/physics/aero.rs
```

## RESPONSE KEYS

```text
STATE: ITEM | STATE | PRIO | TITLE | OWNER | PATHS | DEP | ACCEPT | NEXT
CTX:   OWNER | PATHS | SYMBOLS | CONFIG | TELEMETRY | COMMANDS | STATUS
```

## STATUS

```text
OK       = context sufficient
CTX-MISS = canonical context insufficient; targeted repo discovery allowed
BLOCKED  = dependency prevents execution
```

## AGENT BOOT

```text
TASK-BOOT: STATE(scope,pick) -> CTX(item) -> EXEC
NO-BROAD-DISCOVERY
REPO-DISCOVERY: CTX-MISS only
SUBAGENT=EXECUTOR
```
