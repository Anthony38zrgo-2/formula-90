---
name: problem-solving-guardrails
description: Meta-skill for engineering to detect unproductive debugging loops and enforce evidence-based diagnostic strategies.
---

# Skill: Problem Solving Guardrails

## Purpose
This is a **cross-functional engineering meta-skill**. Its objective is to detect when an agent (or developer) is trapped in a brute-force loop (`Guess -> Modify -> Fail`) and force them to switch to a rigorous diagnostic mode (`Hypothesis -> Experiment -> Fix`). 

**A failed attempt must produce information.** Never iterate code blindly without reducing uncertainty.

## 1. Attempt Budget (Flash vs Pro)
Apply the following rule before iterating code:
- **Attempt 1 (V4 Flash):** A reasonable direct solution based on initial reading or instructions from `instrucciones.txt`.
- **Attempt 2 (V4 Flash):** Requires a formal new hypothesis and gathering additional evidence (logs, telemetry).
- **Attempt 3 (Escalation to V4 Pro):** 🛑 **MANDATORY DIAGNOSTIC MODE**. Flash is prohibited from attempting a third implementation. Execution must stop and escalate to V4 Pro, who must run offline Python tools (`tools/physics_diagnostics/`) to rethink the architecture of the patch.
  - *Physics Context:* If the problem involves car dynamics (rollovers, bouncing, handling), **V4 Pro IS OBLIGATED** to read `docs/game-design/ai_physics_manual.md` before formulating a hypothesis, to avoid hallucinating parameters (e.g., bump stops).
  - 🛑 **TRACK GEOMETRY RULE (The 6th Paradigm):** If a car experiences extreme G-forces or 'Bottom-Out' exclusively when touching curbs, before modifying the car's damping or springs, the agent MUST review the circuit geometry (`CSGPolygon3D`). If the curb has 90-degree walls, it must be smoothed into a ramp.

## 2. Trigger Conditions for Diagnostic Mode
Enter diagnostic mode immediately if:
- The same problem survives 3 attempts (budget exhausted).
- The user repeatedly asks for the same fix.
- A workaround (patch) is being added on top of another previous workaround.
- The next proposed solution relies exclusively on a hunch without measurable evidence.
- A "fix" resolves one symptom but symmetrically breaks another system.

## 3. Diagnostic Mode Workflow (STOP IMPLEMENTATION)
When this mode is activated, stop using code modification tools. You must:
1. **Consult Memory:** Review `docs/troubleshooting/known-issues.md` and `resolved-incidents.md`. Use `git log` on the affected files.
2. **Define State:** Clearly formulate *Observed behavior* vs *Expected behavior*.
3. **Ownership:** Conceptually ask *Who is the true owner of this value/state?* before assuming the bug is in the current script.
4. **Design Experiment & Offline Verification:** Generate a hypothesis. Use the Python verification scripts located in `tools/physics_diagnostics/` to mathematically analyze the specific component (e.g., calculating suspension frequencies) before blindly iterating code.
5. **No Speculative Refactors:** Never rewrite an entire subsystem just because you can't find a small bug.

## 4. Expected Output in Diagnostic Mode
Do not propose code. Respond by structuring your analysis like this:
- **Problem & Unknowns:** 
- **Candidate Hypotheses:** 
- **Next Diagnostic Experiment (No-fix):** 
- **Why this reduces uncertainty:** 

Once you get the experiment result:
- **Root Cause Identified:**
- **Minimal Proposed Fix:**
