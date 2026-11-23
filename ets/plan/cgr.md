# Continuous Git re-integration (CGR)

Each bullet classifies one analyzed source commit from Git or git-for-windows, leading with its bold short hash, an explicit class, and one sentence explaining the behavior and decision.
Entries contain source hashes only.

| Class | Meaning |
| --- | --- |
| `PORTED` | Relevant behavior has been implemented and validated in Gitoxide. |
| `ALREADY-COVERED` | Existing implementation and tests cover the behavior, or a merge adds no behavior beyond separately analyzed commits. |
| `NOT-APPLICABLE` | The Git feature or change has no meaningful counterpart in Gitoxide. |
| `POSTPONED` | Relevant behavior needs infrastructure Gitoxide lacks; name the prerequisite and a concrete revisit condition. |

`POSTPONED` entries remain open for later sweeps; update their existing row when the decision changes or resolves.
Other unresolved analysis remains outside this document.

## Baselines

These local snapshots establish the explicitly requested baseline for future sweeps.
Baseline creation is allowed only when this document is absent; an existing document must retain its recorded baselines.
Commits reachable from either baseline are outside the default sweep scope and remain unexamined; no historical classification is recorded.
Future sweeps process commits reachable from their pinned source tips that are not reachable from either baseline, including newly merged side branches regardless of commit dates, unless other revision bounds are explicitly requested.

| Source repository | Source ref | Full baseline commit |
| --- | --- | --- |
| `git/git` | `refs/heads/master` | `6de20f6092dcf9bdb1c8efe03db4b70c82b423dd` |
| `git-for-windows/git` | `refs/heads/main` | `fa1a8a40e3d68689071e9e88d5a521a6c4ba07a4` |

## Source commits
- **d8e1ba5c4d6f** NOT-APPLICABLE: Share compiled C HTTP sources among Git's Meson executables because Gitoxide builds its HTTP implementation in the shared Cargo crate `gix-transport`.
- **cc839e97a3dc** NOT-APPLICABLE: Reuse objects when building Git's test-only remote helper aliases because Gitoxide dispatches HTTP through `gix-transport` and has no equivalent helper binaries to relink.
- **ae24379dd46b** NOT-APPLICABLE: Precompile Git's C compatibility header for its test helper because Gitoxide builds its shared test helpers as Rust crates.
- **3d7418e351a2** NOT-APPLICABLE: Precompile Git's C unit test headers while compiling Clar separately because Gitoxide uses Rust's test harness and Cargo nextest.
- **47780704699b** NOT-APPLICABLE: Fix the Meson copy order for Git's shell scripts because Gitoxide generates completions directly from its Clap command definitions.
- **e492974847e1** NOT-APPLICABLE: Refresh Git's Meson dependency wrappers because Gitoxide selects native dependencies through Cargo and uses separate Rust implementations for compression and regular expressions.
- **ce4a600322fa** NOT-APPLICABLE: Disable PortableGit's credential helper for GitLab Runner cleanup because Gitoxide's Windows checks run under GitHub Actions and do not use that installer or cleanup path.
- **1c1256682a54** NOT-APPLICABLE: Integrate the Meson series and resolve the Windows CI installer conflict by retaining the credential-helper cleanup without restoring the deleted Rust MSI step because Gitoxide provisions Rust through its GitHub Actions workflow.
- **72c41c3ee0bc** PORTED: Resolve `@{p}` and `@{P}` through the same push-tracking lookup as `@{push}`, including named branches and a following reflog selector.
