# Handoff Protocol

## Purpose
To ensure that execution agents receive precise, verified, and complete instructions without inheriting raw conversational noise or obsolete context.

## 1. Context Bundle Rule
The reasoning agent (Architect) must NEVER hand off vague instructions (e.g., "Fix the physics"). Every execution handoff must include a formal Context Bundle.

## 2. Context Garbage Collection
Before generating the Context Bundle, the Architect MUST run Context GC:
1. Extract VERIFIED FACTS.
2. Preserve CURRENT CONSTRAINTS.
3. Explicitly state UNKNOWNS.
4. Compress REJECTED HYPOTHESES (do not retry without new evidence).
5. Remove SUPERSEDED INFORMATION.
6. Remove RAW TOOL NOISE (terminal logs).

## 3. Mandatory Structure
The Handoff must use the template defined in `handoff_template.md`.

## 4. English Mandate
All Handoff documents (`instrucciones.txt`) MUST be written in strict English to optimize Large Language Model (LLM) performance, syntax precision, and reasoning capabilities.
