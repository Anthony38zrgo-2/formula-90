# Master Guide for AI Agents — Root Project (`formula-90s` / F1 2030)

This document **exclusively** governs the global operational workflows within the repository.
Detailed explanations belong in their respective `docs/` or `.agents/skills/` folders. Do NOT create local `AGENTS.md` files.

---

## 1. Core Principles
1. **Behavior First:** Physics must follow a behavior-first workflow. Read `docs/game-design/vehicle-behavior/`.
2. **Never Guess:** Never guess physics from semantic variable names. Read `docs/game-design/ai_physics_manual.md`.
3. **Attempt Budget:** Respect the Attempt Budget. Same failure twice means STOP and diagnose.
4. **Configuration Authority:** Validate actual runtime configuration using `tools/physics_diagnostics/`.
5. **Evidence:** Do not claim success without verifiable evidence (telemetry, test results).
6. **Handoff:** Use repository-relative paths in handoffs. 

## 2. Skill Routing (Lazy Loading)
Do not load every manual for every task. Use these routing rules:
- **Vehicle handling problem** → load `physics-diagnostics` & `problem-solving-guardrails`
- **.tscn modification** → load `scene-safety` (Godot 4 traps)
- **Repeated failure** → load `problem-solving-guardrails`
- **Planner → Executor transition** → load `context-handoff` & `context-garbage-collection`
- **Physics tuning accepted** → load `regression-validation`
- **Real circuit / Blender track / curbs / guardrails / vegetation** → load `track-reconstruction`

## 3. Context Garbage Collection
Run Context Garbage Collection (see `context-garbage-collection/SKILL.md`) before major agent handoffs, diagnostic escalation, or after repeated failed attempts.
Do not propagate raw historical context when a task-specific Context Bundle can represent the relevant information.

## 4. Scene Safety (`.tscn` Files)
Godot 4 scenes require structural validation. Modifying `.tscn` text directly can delete `node_paths`, silently corrupting references. See `docs/engineering/godot_traps.md` and `scene-safety/SKILL.md`.
