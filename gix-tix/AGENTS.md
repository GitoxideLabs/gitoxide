# gix-tix invariants

## Repository lifetime

- Do not retain a `gix::Repository`, or a platform/object that owns one, in application or event-loop state while tix is idle.
- Open a fresh, non-isolated repository for bounded view population so configuration such as mailmap and diff filters is honored, then retain only detached display data.
- The fill repository may be reused only while continuous navigation is active and must be dropped when its idle timer expires.
- Filesystem watchers retain paths and native watcher handles, never repositories.
