---
name: context-garbage-collection
description: Skill dedicada a la higiene del contexto, limpieza de hipótesis fallidas, y protección del Context Budget.
---

# Skill: Context Garbage Collection

## Purpose
To prevent obsolete, redundant, contradictory, or low-value information from degrading future reasoning. The context window is NOT historical storage.

## The Context GC Pipeline
Run this pipeline before delegating a task to another agent, or when an Attempt Budget expires.

### STEP 1: Extract VERIFIED FACTS
Collect only facts supported by authoritative evidence (code, telemetry, validated documentation).

### STEP 2: Extract CURRENT CONSTRAINTS
Preserve constraints (e.g. "Do not modify aero").

### STEP 3: Compress REJECTED HYPOTHESES
Do not carry full conversation logs. Compress them:
```text
REJECTED: Increased damping to 1.5
EVIDENCE: curb_test_021 showed no improvement.
```

### STEP 4: Remove SUPERSEDED INFORMATION
If `rear_damping` was 5.0 but is now 0.82, completely remove references to 5.0 as the "current state" to avoid confusion. Mark it as `SUPERSEDED`.

### STEP 5: Remove RAW TOOL NOISE
Convert massive terminal outputs into concise conclusions. Delete the raw output from active context.

### STEP 6: Compile the New Context
Pass only the distilled facts, unknowns, and constraints to the next agent via the Context Bundle. Historical information has the LOWEST priority.
