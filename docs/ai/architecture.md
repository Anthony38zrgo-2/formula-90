# Multi-Agent Architecture

## Purpose
This document defines the roles, responsibilities, and constraints of the multi-agent system in the F1 2030 project. It enforces the SRP (Single Responsibility Principle) for agents.

## Core Agents

### 1. Architect / Planner (V4 Pro)
- **Role:** Reasoning, system design, context gathering, and delegation.
- **Responsibilities:**
  - Understand the user objective.
  - Formulate hypotheses based on evidence.
  - Compile the Context Bundle (using Context GC principles).
  - Define the Acceptance Criteria.
  - Delegate execution to the Implementation Agent via `instrucciones.txt`.
- **Constraints:** Must NOT perform massive code changes directly. Must NOT guess physics variables without consulting the Physics Manual.

### 2. Implementation Agent (V4 Flash / DeepSeek)
- **Role:** Code execution, modification, and direct testing.
- **Responsibilities:**
  - Read the Context Bundle provided by the Architect.
  - Read relevant Skills (`scene-safety`, `godot_traps`).
  - Implement the minimal coherent change required to test the hypothesis.
  - Report evidence (telemetry, test results).
- **Constraints:** 
  - Must strictly adhere to the Attempt Budget.
  - Must STOP implementation if the Attempt Budget is exhausted.
  - Must NOT silently broaden the scope of the fix (e.g., modifying aero when fixing suspension).

### 3. Validation Agent (Concept)
- **Role:** Independent verification of changes.
- **Responsibilities:**
  - Assess if code compiles.
  - Check scene structural integrity (using `tscn_parser`).
  - Compare baseline vs candidate telemetry.
- **Outputs:** PASS / FAIL / INCONCLUSIVE.

## Agent Collaboration Workflow
```text
User Request
      ↓
Architect (Context GC + Planning)
      ↓
Context Bundle (`instrucciones.txt`)
      ↓
Executor (Minimal Implementation)
      ↓
Evidence Gathering (Tests / Telemetry)
      ↓
Validator (Baseline vs Candidate)
      ↓
Result (PASS / FAIL / Escalation)
```
