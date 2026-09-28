# src — Validator implementations

Scope: audio analysis and asset validation entry points plus the analyzer implementation.
Consumers: content gates run from `scripts/`.
Rules: exit codes are the contract: zero accepts, nonzero reports. Human-readable detail goes to stdout, never into the validated files.
