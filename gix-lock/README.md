Use lock-files in the way git does with auto-cleanup being the most notable feature.

The `unstable` feature exposes the `backoff` and `tempfile` plumbing re-exports outside the compatibility guarantee.
Pin an exact crate version when using those re-exports. Lock acquisition and resource updates are available without them.
The public `gix-error` dependency still needs stabilization; this pilot does not claim that it is stable already.

* [x] writable lock files that can be committed to atomically replace the resource they lock
* [x] read-only markers that lock a resource without the intend to overwrite it
* [x] auto-removal of the lockfiles and intermediate directories on drop or on signal
