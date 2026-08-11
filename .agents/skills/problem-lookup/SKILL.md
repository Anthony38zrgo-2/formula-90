---
name: problem-lookup
description: Query indexed Formula-90 failures by error or failure signature before repeating an implementation, stacking a workaround, or starting an expensive diagnostic loop.
---

# Problem Lookup

Run `agentdb problem <signature-or-term>` when an error matches a known pattern, before Attempt 2, when a workaround is proposed, or when a documented project trap may apply.

Search using the observable signature rather than a guessed fix. A hit is evidence to inspect, not authority to apply the historical solution. Verify ownership, configuration, and signature equivalence.

Record whether the lookup supports, falsifies, or does not affect the active hypothesis. Do not count a renamed repeat of a known failed approach as a new hypothesis.
