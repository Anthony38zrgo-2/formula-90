# profiles — Agent profiles

Scope: JSON profiles binding task types to skill sets, selected through `active-profile.json`.
Consumers: agents at task start and `.agents/tools/profile.py`.
Rules: adding a skill to the catalog requires updating the profiles that should offer it.
