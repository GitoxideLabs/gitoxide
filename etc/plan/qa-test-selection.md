# Investigating LSP-based QA test selection

Investigated on 2026-11-04, at `61bbd5b`, using `rust-analyzer 1.99.1
(58cf8f9c 2026-10-13)` on macOS ARM64. The aim was to reduce demand-driven QA
below package/configuration granularity without maintaining a historical coverage
map or implementing Rust dependency analysis ourselves.

**The available APIs can suggest tests, but the experiment does not justify
omitting tests from QA.** Symbol resolution and a complete inverse execution graph
are different capabilities. Ordinary code in this repository exposes missing
edges, even when the server reports healthy, completed analysis. No LSP selector
was added to the QA runner.

## Availability and APIs

The standard GitHub runner installation scripts for
[Ubuntu](https://github.com/actions/runner-images/blob/main/images/ubuntu/scripts/build/install-rust.sh),
[macOS](https://github.com/actions/runner-images/blob/main/images/macos/scripts/build/install-rust.sh), and
[Windows](https://github.com/actions/runner-images/blob/main/images/windows/scripts/build/Install-Rust.ps1)
install minimal stable Rust plus rustfmt/clippy, without rust-analyzer or rust-src.
Installation is straightforward:

```sh
rustup component add --toolchain stable rust-analyzer rust-src
rustup run stable rust-analyzer
```

Selecting the toolchain explicitly avoids a project override choosing another
installation. See the [Rustup profiles](https://rust-lang.github.io/rustup/concepts/profiles.html)
and [rust-analyzer installation](https://rust-analyzer.github.io/book/installation.html) docs.

The probe used these supported LSP methods/extensions:

- `textDocument/prepareCallHierarchy` and recursive `callHierarchy/incomingCalls`.
- `rust-analyzer/relatedTests`, which finds direct tests and follows some test
  helpers, but does not recursively follow arbitrary production callers.
- `experimental/runnables` without a position, to discover current tests in a file.
- `textDocument/references`, `documentSymbol`, `declaration`, and `definition` to
  investigate edges missing from call hierarchy.

The [LSP extensions](https://rust-analyzer.github.io/book/contributing/lsp-extensions.html)
also document `experimental/discoverTest` for test-tree discovery. That API was
not exercised. The probe waited for `experimental/serverStatus` to report
`quiescent: true`; the results below came from runs reporting `health: "ok"`.
This status establishes readiness, not semantic completeness.
Full-workspace stderr also contained cyclic-dependency warnings, including self
dev-dependencies, despite the healthy status. The small counterexample workspace
did not rely on those cycles.

## Real workspace results

Candidate tests were matched by source location to ordinary test runnables,
excluding doctests and Insta snapshot-update actions. Package, target and exact
test name were then checked against a freshly compiled Nextest inventory:

```sh
cargo nextest list --locked --message-format json \
    -p gix-archive -p gix -p gix-diff
```

That inventory contained 750 tests. The table counts candidates present in those
three packages; it does not assert that they are all affected tests.

| Changed symbol | Exact candidates | Expand to enclosing test module |
| --- | ---: | ---: |
| `gix_archive::write_stream` / `write_stream_seek` | 5 | 64 |
| `gix::Repository::worktree_archive` | 1 | 60 |
| `gix::Repository::workdir_path` | 2 | 39 |
| `gix_diff::blob::diff_with_slider_heuristics` | 10 | 16 |
| `gix_diff::tree::Recorder` implementation of `Visit::visit` | 0 | 0 |
| `gix_diff::tree::Visit::visit` declaration | 0 | 0 |
| CLI `verify_mode` helper | 0 | 0 |

The slider query additionally found eight `gix-tix` candidates, or 124 after
module expansion. That package was outside the compiled inventory, so those
candidates were not validated. Counts across all discovered packages were 18/140.

Module expansion includes descendants of the selected test's enclosing module
in the same package and target; a test at the crate root stays exact. It greatly
broadens the archive case and cannot recover a module that traversal never
reached. Exact selection would be preferable if the underlying relation were
complete. Expanding modules does not make the current relation trustworthy.

The archive candidates also illustrate over-selection: the internal-format tests
share helper code with tar/zip tests, but return without calling the archive
writers. Static reachability is not runtime coverage.

The CLI has Rust library tests, despite disabling the `gix`/`ein` binary test
harnesses. However, `verify_mode` is exercised through shell journeys invoking
the executable. A Rust call graph does not cross that process boundary.

## Concrete missing edges

A separate two-member Cargo workspace in this worktree exercised known paths.
Both its no-default and alternate-feature configurations passed all 12 tests.
Changing the leaf and re-exported function's return values then failed eight
tests, including the macro-generated, function-pointer and renamed-export tests
missing from basic traversal. The fixture source was restored afterwards.

| Path from test to changed function | Observed result |
| --- | --- |
| Direct call | Found |
| Production wrapper or test helper | Recursive incoming calls found it; `relatedTests` alone was insufficient |
| Generic call through a trait | Implementation had no callers; querying its trait declaration recovered the generic caller |
| Function pointer stored in a static | Call hierarchy missed it; references plus enclosing-symbol traversal recovered it |
| Implicit `Drop` or overloaded `Add` | Call hierarchy missed it; widening to uses of the concrete self type found candidates |
| Test generated by `macro_rules!` | Runnable discovery found the test, but both incoming calls and references from the changed function missed its call |
| Renamed function re-export | Found the test using the original name, missed the test using the alias |

Trait declaration navigation is supported, but rust-analyzer documents the
[generic/trait call-hierarchy limitation](https://github.com/rust-lang/rust-analyzer/issues/19358).
Reference traversal and widening impls to types recover some cases without
parsing Rust. They still require custom traversal policy; the broad owner
experiment reached its 250-symbol cap through standard-library traits. A drained
queue or a completed request cannot certify that no edge was omitted.

The re-export problem occurs in real `gix-diff`: `tree::function::diff` is exported
as `gix_diff::tree`. Traversal from the `Visit` declaration reached `diff` and
stopped before integration-test callers. The relevant tests are under
`tree::changes::to_obtain_tree`, using `diff_commits` and
`diff_with_previous_commit_from`. Querying `diff_commits` directly found its two
tests; querying `diff` or its re-export found none. Thus even the trait-declaration
workaround did not recover these callers.

## Features must match the execution configuration

Cargo metadata describes a different feature union from a selected package's
test build. This is reproducible in the real workspace:

```sh
cargo metadata --locked --offline --format-version 1 \
    --manifest-path gix-archive/Cargo.toml --no-default-features --features sha1
cargo tree --locked --offline -p gix-archive --no-default-features \
    --features sha1 --depth 0 --format '{p} {f}'
```

Metadata enables `default,sha1,tar,tar_gz,zip` on `gix-archive`; the selected-package
tree enables only `sha1`. Rust-analyzer takes active features from metadata.
Linking the member manifest and requesting no default features did not fix this.

In the small workspace, another member enabled the subject's `alternate`
feature. Rust-analyzer exposed `feature_enabled` and could not resolve the
`cfg(not(feature = "alternate"))` function, while the actual package-scoped test
run contained `feature_disabled`. This was a healthy server with the requested
no-default-features configuration, not a stale coverage list.

All-features analysis cannot cover negated cfg branches or mutually exclusive
features. Platform cfgs, build-script output, proc macros and test targets also
matter. Cargo's unstable [unit graph](https://doc.rust-lang.org/cargo/reference/unstable.html#unit-graph)
can supply authoritative build-unit features. Translating it into
[rust-project.json](https://rust-analyzer.github.io/book/non_cargo_based_projects.html)
could avoid writing our own feature resolver, but needs an adapter and the other
compiler inputs. This route was not implemented and would not repair missing
inverse call edges.

## Cost and fresh tests

Full-workspace sessions used offline Cargo, `checkOnSave: false`, all targets,
build scripts/proc macros enabled, and `cargo.cfgs: ["debug_assertions", "!miri"]`.
These are exploratory local observations with warm build caches, not repeated
benchmark distributions or measurements on GitHub runners:

| Configuration | Ready | Archive query after readiness | Slider query after earlier queries |
| --- | ---: | ---: | ---: |
| Eager cache priming, six threads | 81.2 s | 1.4 s | 8.5 s |
| Lazy cache priming | 65.6 s | 7.2 s | 8.8 s |
| Lazy, build scripts and proc macros disabled | 33.1 s | 6.2 s | 8.4 s |

Queries include traversal and runnable discovery, not just one LSP request.
Progress logs showed roughly 33 seconds per scan of 404 source roots; a session
with build scripts/proc macros enabled scanned twice. Disabling those inputs is
a lower-information experiment, not an acceptable completeness optimization.
A persistent server might amortize startup, but neither incremental updates nor
per-configuration reuse were benchmarked. Test filtering also leaves compilation,
listing, clippy, doctests and journeys to pay for, so candidate counts alone do
not predict an overall QA speedup.

Appending a new direct test and restarting rust-analyzer discovered it immediately
in both the incoming-call candidates and the file's runnable inventory. Fresh
discovery is therefore possible without a maintained coverage map. This tested
an addition in an existing file, not discovery of a new target or file. A usable
selector must independently enumerate current tests and include new/changed
tests; collecting runnables only in files reached from a production change is
insufficient. Deleted code also needs information from the base revision.

A practical LLVM-based per-test runtime coverage approach uses an instrumented
build, runs each test separately with a unique `LLVM_PROFILE_FILE` output-path
pattern, then merges/exports its profiles with LLVM tools. Identify results by configuration,
test target and test name, not just process ID. Fresh collection requires running
the tests being measured, so it cannot itself make that same full run cheaper.
Reuse would introduce the invalidation problem this investigation aimed to avoid.

## Outcome

Keep package/configuration selection as the authority for omission. LSP results
could prioritize tests, but adopting that now adds material startup cost and a
new maintained tool without a demonstrated end-to-end saving. Revisit if an
upstream semantic dependency API can represent the missing edges and the exact
Cargo build configuration. No automatic module expansion or historical test map
is needed for the existing QA implementation.

Local probe source, JSON inputs/results, the small fixture and test logs are
retained under ignored `target/qa-lsp` in the investigation worktree. The probe
is deliberately not a shipped QA component. Its protocol/containment tests and
Clippy passed; those checks validate the client, not the soundness of selection.
