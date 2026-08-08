---
name: context-handoff
description: Protocolo para generar Context Bundles formales entre el Planner (Pro) y el Executor (Flash).
---

# Skill: Context Handoff

## Purpose
The planning/reasoning agent must never hand off vague instructions. Every execution handoff must include a Context Bundle. This prevents the Executor from guessing paths, variables, or user intent.

## The Context Bundle Structure
When creating `instrucciones.txt`, you MUST use the following headers (use `docs/ai/handoff_template.md` as reference):

1. **# OBJECTIVE**
2. **# OBSERVED PROBLEM**
3. **# EXPECTED BEHAVIOR**
4. **# CURRENT EVIDENCE**
5. **# RELEVANT FILES**
6. **# RELEVANT SYMBOLS**
7. **# DEPENDENCIES**
8. **# KNOWN CONSTRAINTS**
9. **# DO NOT MODIFY**
10. **# REJECTED APPROACHES**
11. **# VALIDATION COMMANDS**
12. **# ACCEPTANCE CRITERIA**
13. **# ATTEMPT BUDGET**
14. **# ROLLBACK / FAILURE CONDITIONS**

## Rules
- **Paths:** Use repository-relative paths (e.g., `game/scenes/vehicles/f1_2026_car.tscn`).
- **Language:** The Context Bundle MUST be written in strict English (The English Mandate).
- **Specificity:** Do not say "update physics". Say "change `rear_damping_ratio` from 5.0 to 0.78".
