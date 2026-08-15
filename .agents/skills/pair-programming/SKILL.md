---
name: pair-programming
description: Facilitate collaborative pair programming after completing a backlog task or technical milestone. Summarize deliverables, present affected files and validation evidence, and explicitly prompt the user for code review, design feedback, or approval before proceeding to the next activity.
---

# Pair programming: Backlog closeout and code review gate

Use this skill whenever a backlog item, micro-patch, or technical milestone reaches completion. It ensures that the human driver and AI navigator maintain shared context, verify code quality together, and formally sign off on changes before advancing.

## Closeout Protocol

Upon completing any backlog task:

1. **Synthesize Deliverables**:
   - Backlog ID and title (e.g., `BG3-001: Congelar procedencia de assets v3`).
   - Clear statement of what was accomplished and what remains untouched.
2. **Expose Changed Surface**:
   - List created, modified, or deleted files with clickable markdown links (`[file](file:///...)`).
   - Highlight any non-obvious design decisions, trade-offs, or contracts introduced.
3. **Show Validation Evidence**:
   - Deterministic test/validation results (`PASS`, `FAIL`, `INCONCLUSIVE`).
   - State of hashes, dimensions, lint/syntax checks, or smoke tests.
4. **Offer Code Review & Next Action**:
   - Prompt the user with a focused review question rather than silently jumping to the next task.

## Mandatory Review Prompt

At the conclusion of each backlog activity, end the report with a structured pair-programming gate:

```text
---
### Revisión de Código y Siguiente Paso (Pair-Programming Gate)

- Tarea completada: `[TASK-ID]` - [Título de la tarea]
- Archivos afectados:
  - [NEW/MOD/DEL] [nombre_archivo](file:///path/to/file)
- Estado de validación: `PASS` | `INCONCLUSIVE`
- Próxima tarea sugerida en backlog: `[NEXT-TASK-ID]` - [Título de la siguiente tarea]

¿Deseas revisar el diff/código en detalle, realizar algún ajuste, o procedemos directamente con `[NEXT-TASK-ID]`?
```

## Review Guidelines

- **Interactive Navigation**: If the user requests a code review, provide a surgical walkthrough focusing on architecture, edge cases, error handling, and conformance with repository rules.
- **Immediate Feedback Loop**: Address user feedback on the current task before starting the next one.
- **Rollback Readiness**: If the user rejects the approach or points out regressions, revert cleanly or apply a micro-patch before modifying backlog state.
- **Prevent Blind Progress**: Never start the next backlog task automatically without user confirmation when code changes were introduced.
