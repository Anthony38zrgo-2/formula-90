# schemas — Agent state schemas

Scope: SQL schemas for agent bookkeeping and runtime state.
Consumers: agent tooling persisting handoffs and diagnostics.
Rules: schema changes migrate consumers in the same change; never widen a schema for one caller.
