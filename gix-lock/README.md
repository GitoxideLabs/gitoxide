Use lock-files in the way git does with auto-cleanup being the most notable feature.

The `unstable` feature exposes the `backoff` and `tempfile` plumbing re-exports outside the compatibility guarantee.
Pin an exact crate version when using those re-exports. Lock acquisition and resource updates are available without them.
Directory creation and its retry configuration use stable APIs internally; neither requires `unstable`.
Acquisition errors use the stable `gix-error::Error` type and its lock recovery predicates.
Experimental error construction helpers are used internally with an exact dependency version.

* [x] writable lock files that can be committed to atomically replace the resource they lock
* [x] read-only markers that lock a resource without the intend to overwrite it
* [x] auto-removal of the lockfiles and intermediate directories on drop or on signal
