# Implementation sources and licensing

The normative source for this implementation is Git's published
`Documentation/technical/reftable.adoc` at commit
`f78ce2f7b6df702f93d40b85d6bda92a3f65da79`.

The Rust format, stack, adapter, tests, and fixtures were written from that
specification. Compatibility tests compare their behavior with Git's executable
and its `t0610` through `t0614` test suite. Bidirectional format tests were also
run against these independently maintained implementations at the listed
revisions:

| Implementation | Revision |
|---|---|
| JGit | `db8cba30cdb029ff8ef4f52824f4782e3c4077f7` |
| Sley | `41a1c8373c6f3eb91d295694dd5d5dcbb7f48a59` |
| Dulwich | `02540a8a9c12a2776cf507993b180bbc595e003b` |
| `hanwen/reftable` | `ca20b64f41b87a559a0ad0b8478484b827278704` |

No source or tests from Git, JGit, Sley, Dulwich, Google's reftable library,
`hanwen/reftable`, or earlier Gitoxide reftable pull requests were copied or
transliterated. Cross-reader harnesses were independently authored in temporary
directories and were not committed. Git- and peer-generated tables are test
data for interoperability testing, not imported source code.

`gix-reftable` is distributed under `MIT OR Apache-2.0`, as declared in
`Cargo.toml`. Both `LICENSE-MIT` and `LICENSE-APACHE` are included in the crate
package. Third-party Rust dependencies retain their own licenses and are
identified by `Cargo.lock` and Cargo package metadata; no additional vendored
source or notice file is introduced by this crate.
