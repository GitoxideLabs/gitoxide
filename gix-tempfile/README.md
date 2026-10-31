Use tempfiles to minimize the risk of resource leakage when preparing to overwrite or create a file with new content
in a signal-safe way, making the change atomic.

Tempfiles can also be used as locks as only one tempfile can exist at a given path at a time.

Only the convenience `create_dir` and `remove_dir` re-exports are excluded from the compatibility guarantee
and require the `unstable` feature. Pin an exact crate version when using these re-exports. File creation,
including parent-directory creation through `ContainingDirectory::CreateAllRaceProof` and its public
`gix_fs::dir::create::Retries` configuration, cleanup, persistence, and signal support are stable APIs available
without `unstable`. Signal support requires the `signals` feature. Public types from the `tempfile` dependency
remain part of the stable contract.

* [x] registered temporary files which are deleted automatically as the process terminates or on drop
    * [x] write to temporary file and persist it under new name
    * [x] close temporary files to convert them into a marker while saving system resources
    * [x] mark paths with a closed temporary file
* [x] persist temporary files to prevent them from perishing.
* [x] signal-handler integration with `gix` to clean lockfiles before the process is aborted.
* [x] use a temporary file transparently due thanks to implementations of `std::io` traits
