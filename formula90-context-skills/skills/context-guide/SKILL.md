# SKILL: context-guide

```text
ALIAS: CTX
ROLE: technical ownership/context resolver
INPUT: backlog item | area | subject | symbol | path
OUTPUT: minimal execution context
```

## INPUT

```text
/ctx <ITEM>
/ctx AREA=<AREA> SUBJECT=<subject>
/ctx SYMBOL=<symbol>
/ctx PATH=<path>
```

## FLOW

```text
resolve -> ownership -> paths -> symbols -> config/telemetry -> execution hints
```

## OUTPUT

```text
OWNER=<module>
PATHS=<relevant paths>
SYMBOLS=<relevant symbols>
CONFIG=<config SOT|NONE>
TELEMETRY=<telemetry SOT|NONE>
COMMANDS=<known narrow commands|NONE>
STATUS=OK|CTX-MISS
```

## POLICY

```text
EXACT-FIRST
MINIMAL-CONTEXT
NO-BROAD-DISCOVERY
NO-NEIGHBOR-SCAN
NO-REPLAN
```

## DISCOVERY-GATE

```text
STATUS=OK       -> EXEC
STATUS=CTX-MISS -> targeted exact-symbol/path search allowed
BROAD-REPO-SCAN -> prohibited
```

## EXAMPLES

```text
/ctx AE-014
/ctx AREA=SUS SUBJECT=preload
/ctx SYMBOL=AeroModel
```
