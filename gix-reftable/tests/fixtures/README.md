# Reftable interoperability fixtures

`../git_interop.rs` creates its fixtures at test time with the `git` executable
under test. It starts a files-backed repository, creates direct, peeled,
symbolic, and reflog data, runs:

```sh
git refs migrate --ref-format=reftable
```

and then deletes a migrated branch so Git writes a real reftable tombstone.
The reader assertions compare exact direct IDs, symbolic targets, annotated-tag
target and peeled IDs, and the deletion value rather than checking names alone.

The SHA-1 case creates 800 additional references so Git emits multiple ref
blocks plus its ref/object indexes. The SHA-256 case exercises the version 2
header and 32-byte records. The reverse fixture replaces Git's generated stack
with a deterministic table written by `gix-reftable`, then asks Git to resolve
names, read reflogs, and perform an object-to-reference lookup.

Fixtures are generated instead of checked in so the test records compatibility
with the Git version actually used by CI. Every generated repository contains
`reftable-fixture.provenance` at its root with `git --version` output and the
ordered command sequence that produced it (including the number of commands
fed to `git update-ref --stdin`). The test skips only when that Git cannot
initialize a reftable repository at all.
