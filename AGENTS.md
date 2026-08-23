# Formula-90 Agent Protocol

CANON SCRIPT:

run_f1_94.ps1

PIPELINE:

sprint planning -> backlog item -> implement -> review -> human gate -> sprint retrospective -> done

REBUILD SAFETY: Never mix unrelated changes or create large WIP commits. When performing a full rebuild, never reuse DLLs, object files, or ignored caches from another branch. Verify HEAD, clean Cargo/SCons/game/.godot, and confirm that the resulting BUILD matches the current source state.

PROVENANCE: Before modifying anything, record and verify the current branch, HEAD, and git status. Never switch, reset, or modify a dirty branch without first inventorying its changes and creating an approved backup.

COMMIT SCOPE: Keep commits atomic and stage explicit paths only. git add -A is prohibited. Never include pre-existing, generated, unrelated, or foreign changes without reviewing git diff --cached.

RUNTIME PARITY: The runtime must reject binaries whose BUILD does not match HEAD. Every launch script must either compile the required binaries or validate BUILD/HEAD parity before starting Godot.
