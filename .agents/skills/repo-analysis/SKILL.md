---
name: repo-analysis
description: Procedure for auditing and diagnosing the repository before proposing changes (Ideal for Antigravity).
---

# Skill: Repo Analysis

## Purpose
Exhaustively analyze the state, dependencies, and root cause of a problem before proposing or executing code changes.

## Workflow
1. Read `AGENTS.md` (root) to understand the unbreakable rules.
2. Read `PROJECT_STATE.md` to know the current state and technical debt.
3. Review recent commits and test logs (`tests/`).
4. Identify which subsystem is the owner of the defective behavior.
5. Trace the execution flow (GDScript vs C++).
6. **Consult Troubleshooting:** Review `docs/troubleshooting/known-issues.md` and `resolved-incidents.md` before assuming it's a new bug.

## Constraints
- **DO NOT** propose massive changes without isolating the root cause first.
- **Diagnostic Mode:** If you find yourself in diagnostic mode (after multiple failures), stop the implementation. Refer to `.agents/skills/problem-solving-guardrails/SKILL.md` and apply the experimental flow detailed there.
- Avoid creating parallel solutions or duplicating logic (e.g., do not create a second `VehicleController` if one already exists).

## Format (Suggested Output)
When analyzing a problem, deliver your report structured like this:
- Root cause:
- Relevant files:
- Proposed change (reasonable minimum):
- Impact & Risks:
- Architectural solution proposal (avoiding hardcode).
- Necessary tests.

## Handoff / Transfer
Once you have finished your analysis and generated an implementation plan, **you must dump your design into the `instrucciones.txt` file** in the root of the project. This file will serve as the mandatory *prompt* for the agent in charge of massive programming (e.g., DeepSeek).

### 🛑 THE ENGLISH MANDATE
**ABSOLUTE RULE:** Antigravity (V4 Pro) is STRICTLY PROHIBITED from writing the `instrucciones.txt` file in Spanish. Large Language Models perform exponentially better when reading instructions in English. You **MUST** translate the entire handoff prompt to English before saving the file.

### Context-Bundle Rule (Anti-Hallucinations)
When writing `instrucciones.txt`, V4 Pro is **prohibited** from leaving a purely conceptual plan. You must inject an explicit "Context-Bundle" so that the executing agent (Flash) does not have to guess anything. This bundle must include:
1. **Absolute Paths:** The exact name and path of the files to modify (e.g., `game/scenes/ui/main_menu.tscn`).
2. **Relevant Structure:** If a scene is going to be modified, include the internal path of the node (e.g., `Path = $HUD/MarginContainer/Label`).
3. **Function Signatures:** Write out literally the name of the function that Flash must edit or inject (e.g., `func _on_button_pressed():`).
4. **Dependencies:** If the script assumes the existence of an Autoload, explicitly name it and give its path.
