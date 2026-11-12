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
