# `gix-reftable`

`gix-reftable` is Gitoxide's specification-driven implementation of Git's
reftable format. It reads and deterministically writes immutable version 1 and
version 2 tables, and manages crash-safe `tables.list` stacks with locked
additions, verification, cleanup, compaction, and reflog expiry.

The crate supports SHA-1 and SHA-256 through separate Cargo features. It is a
storage engine: repository discovery and Git reference semantics live in
`gix` and `gix-ref` respectively.

```rust
use gix_reftable::{Limits, SnapshotOptions, Stack};

fn inspect() -> Result<(), Box<dyn std::error::Error>> {
    let table = gix_reftable::Table::read("path/to/table.ref", Limits::default())?;
    let _branch = table.find_ref(b"refs/heads/main");

    let stack = Stack::open(
        "path/to/reftable",
        gix_hash::Kind::Sha1,
        SnapshotOptions::default(),
        Limits::default(),
    )?;
    let _snapshot = stack.snapshot()?;
    Ok(())
}
```

See [COMPATIBILITY.md](COMPATIBILITY.md) for the conformance evidence,
[PERFORMANCE.md](PERFORMANCE.md) for the recorded benchmark baseline, and
[IMPLEMENTATION-SOURCES.md](IMPLEMENTATION-SOURCES.md) for the specification,
comparison implementations, and licensing details.
