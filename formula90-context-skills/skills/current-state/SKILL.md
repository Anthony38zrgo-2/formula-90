# SKILL: current-state

```text
ALIAS: STATE
ROLE: backlog/state resolver
SOT: GitHub backlog/project metadata
OUTPUT: compact structured context
```

## INPUT

```text
/state <AREA>:<PICK>
/state <ITEM>
/state <ITEM> DETAIL
```

## FLOW

```text
parse -> query SOT -> filter AREA/ID -> apply PICK -> return minimal item context
```

## PICK

```text
TOP|READY|ACTIVE|LATEST|BLOCKED|ALL
```

## OUTPUT

```text
ITEM=<AREA-NNN>
STATE=<state>
PRIO=<priority>
TITLE=<title>
OWNER=<owner>
PATHS=<known paths>
DEP=<dependencies|NONE>
ACCEPT=<acceptance criteria>
NEXT=<next causal action>
```

## POLICY

```text
CTX-FIRST
NO-REPO-SCAN
NO-BROAD-DISCOVERY
DEFAULT=MINIMAL
DETAIL=EXPLICIT-ONLY
```

## FAIL

```text
NO-MATCH -> CTX-MISS
AMBIGUOUS -> return candidates only
NO-SOT -> CTX-MISS
```

## EXAMPLES

```text
/state AE:READY
/state SUS:TOP
/state AE-014
/state AE-014 DETAIL
```
