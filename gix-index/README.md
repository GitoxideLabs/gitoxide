## What it is
Library crate for parsing, reading, and updating the Git index (dircache) and querying file state used by higher‑level operations in gitoxide.

## When to use / When not to
- **Use when** you need direct access to the Git index for analysis/synchronization/status tooling.
- **Do not use when** you need an end‑user CLI or a high‑level library to manage an entire repository; this is a focused building block.

## Related crates
- `gix-worktree` — shared types and utilities for worktrees.
- `gix-status` — compute repository status and file changes.

## Links
- crates.io: https://crates.io/crates/gix-index
- docs.rs: https://docs.rs/gix-index/latest/gix_index/

## Stability & MSRV
Stability: Unspecified — see the project‑wide stability policy (https://github.com/GitoxideLabs/gitoxide/blob/main/STABILITY.md).
MSRV: Inherits the workspace’s Minimum Supported Rust Version — see MSRV policy for details (https://github.com/GitoxideLabs/gitoxide/blob/main/.github/workflows/msrv.yml).

## License
Dual-licensed under MIT OR Apache-2.0.

## Developer notes

### Test fixtures

Most of the test indices are snatched directly from the unit test suite of `git` itself, usually by running something like the following

```shell
 ./t1700-split-index.sh -r 2 --debug 
```

Then one finds all test state and the index in particular in `trash directory/t1700-split-index/.git/index` and can possibly copy it over and use as fixture.
The preferred way is to find a test of interest, and use its setup code within one of our own fixture scripts that are executed once to generate the file of interest.
