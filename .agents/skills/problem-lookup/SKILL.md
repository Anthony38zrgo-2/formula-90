---
name: problem-lookup
description: Query indexed known failures before repeating a fix or starting a high-cost diagnostic loop.
---

# Problem Lookup

Run `agentdb problem <signature-or-term>` when:
- an error matches a familiar pattern;
- the first implementation attempt fails;
- a workaround is being stacked on another workaround;
- a task touches a documented project trap.

A hit is evidence to inspect, not permission to apply a fix blindly. Verify symptoms and ownership before changing code.
