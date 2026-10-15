# banks — Runtime sound banks

Scope: the runtime bank set: the shared commons bank, the V10 baseline, the AI-only V10 companion, the inactive Ferrari V8 candidate, and the frozen legacy bank.
Consumers: the vehicle audio engine, resolving banks from the filesystem relative to the Godot project root.
Rules: no new runtime bank without retiring or superseding an old one explicitly. Each bank documents itself in its own AGENTS.md.

The geometric F1 2030 profile selects v10-v2-bank. The inactive v8-bank candidate preserves the same non-engine events and transmission recording for later comparison. The AI-only v10-bank-ai companion coexists explicitly without superseding v10-v2-bank: artificial-driver voices alone select it under the capped AI voice policy, and the player bank stays canonical.
