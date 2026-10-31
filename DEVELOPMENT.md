# Development Guide

See the [contribution guide] for requirements to meet before beginning implementation.
Follow the [repository-wide development rules](AGENTS.md); this guide provides additional workflow, tooling, and design background.

[contribution guide]: https://github.com/GitoxideLabs/gitoxide/blob/main/CONTRIBUTING.md

## Common commands

Run `just` to browse commands grouped by purpose, with everyday development tasks
first. Use `just --groups` to list the groups, or filter the overview with, for
example, `just --list --group 'Dependencies and SBOMs'`.

## Release source paths

The GitHub release workflow builds `gix` and `ein` through
`etc/scripts/remap-release-paths.sh`. It appends rustc's `--remap-path-prefix`
flags for all target crates, including dependencies, using stable virtual roots:
`/gitoxide` for the checkout, `/cargo` for Cargo sources, `/rust` for the selected
Rust toolchain, and `/generated` for build output. Home, Rustup, and temporary
paths are also remapped. Native host paths, canonical aliases, Windows separator
variants, and `cross` container mounts are covered. Normal
local development builds are unchanged.

To opt into the same policy locally, run these commands from the workspace root:

```sh
bash etc/scripts/remap-release-paths.sh build cargo build --release --locked -p gitoxide --bins
bash etc/scripts/remap-release-paths.sh check target/release/gix target/release/ein
```

## Practices

 * **fuzzing**
   * fuzz parsers, algorithms, and other code that processes untrusted input.
   * add a regression test for every issue found before fixing it.
 * **benchmarks**
   * benchmark performance-sensitive changes before and after implementation.
   * use representative inputs and compare against git when applicable.
 * Adhere to the [stability guide](https://github.com/GitoxideLabs/gitoxide/blob/main/STABILITY.md)

## Commit History

We use feature branches and pull requests to take advantage of CI and GitHub review tools, and merge with merge commits
to make individual efforts stand out. There is no requirement to squash commits or linearize history.
However, commits may be squashed when the history is too messy to review, and commit titles may receive minor edits.

Use Tix to organize topic-oriented commit stacks. See the [Tix workflow](.agents/skills/tix/SKILL.md) for history edits
and preservation of review state.

## Configuration and overrides

As a general rule, respect and implement all applicable [git-config](https://git-scm.com/docs/git-config) by default, but allow the
caller to set overrides. How overrides work depends on the goals of the particular API so it can be done on the main call path,
forcing a choice, or more typically, as a side-lane where overrides can be done on demand.

Note that it should be possible to obtain the current configuration for modification by the user for selective overrides, either
by calling methods or by obtaining a data structure that can be set as a whole using a `get -> modify -> set` cycle.

Note that without any of that, one should document that with `config_snapshot_mut()` any of the relevant configuration can be
changed in memory before invoking a method in order to affect it.

Parameters which are not available in git or specific to `gitoxide` or the needs of the caller can be passed as parameters or via
`Options` or `Context` structures as needed.

## Software bills of materials (SBOMs)

Install the pinned Rust tools once with `just sbom-install`, then run `just sbom` to
write CycloneDX 1.5 and SPDX 2.3 JSON inventories of all workspace members, with all
features and platforms enabled. `cargo deny` remains the audit tool; it does not
export these SBOM formats. `cargo-cyclonedx` generates the inventory and
`sbom-tools` converts that same inventory to SPDX, reporting any metadata loss
(such as CycloneDX properties) on standard error.

Use `--package` to select a workspace crate with its default features on the host
platform. The `gitoxide` package builds the `gix` and `ein` binaries; the `gix`
package is the library. For example:

```sh
just sbom --package gitoxide --no-default-features --features small
just sbom --package gitoxide --no-default-features --features max-pure
just sbom --package gix --no-default-features --features sha1
just sbom --package gix --features blocking-http-transport-reqwest-rust-tls
just sbom --package gix --no-default-features --features sha1,blocking-http-transport-curl-openssl
```

`--features` accepts comma- or space-separated named crate features and can be
repeated. `--all-features` enables every feature of the selected crate. Use
`--target TRIPLE` for another platform or `--target all` for all platforms.

Outputs default to `<Cargo target directory>/sbom/<workspace-or-package>.cdx.json`
and `.spdx.json`. Use `--output-dir PATH` to keep inventories for different feature
or target selections alongside each other. Each run replaces the existing files
for that package or workspace only after both tools succeed.

Inventories include runtime and build dependencies and exclude dev-only dependencies.
Every workspace member is a root in workspace mode, including internal tooling.
Package mode resolves in a temporary workspace so unrelated workspace members
cannot enable extra HTTP or TLS backends. Resolution uses a copy of `Cargo.lock`
and rejects changed package versions; source manifests and lockfiles stay untouched.
The selected graph comes from `cargo tree`, which resolves platform-specific features
and host build dependencies that `cargo metadata --filter-platform` alone cannot isolate.
These are Cargo dependency inventories, not scans of native libraries installed
on the system or of the contents of a compiled binary.

Run `just sbom-test` to exercise generation using an isolated offline Cargo fixture,
including feature selection and matching inventories in both formats.

## General

* **async**
  * **library client-side**
    * Provide `async` network clients as opt-in using feature toggles, and document supported transports and runtime requirements.
  * **User Interfaces**
    * User interfaces can greatly benefit from using async as it's much easier to maintain a responsive UI thread that way thanks
      to the wonderful future combinators.
    * Offload blocking work, including pack processing in async APIs, to an execution context that tolerates blocking rather than the UI thread.
    * Ignoring a result does not stop background work or roll back filesystem changes. Inspect the operation's documented interruption outcome.
  * **server-side**
    * Keep async integration independent of a particular runtime where feasible; account for CPU-intensive and blocking work.
  * **shared blocking and async code**
    * Use async-shaped source with the `keep`, `discard`, and `sync` attributes from `gix-macros`. Alias them in a local `bisync` module at each
      integration point so both variants can coexist while being generated from one implementation that cannot drift.
* **`Default` trait implementations**
  * These can change only if the effect is contained within the callers process.
    This means **changing the default of a file version** is a **breaking change**.
* **Using the `Progress` trait**
  * When receiving a `Progress` implementation
     * without calling `add_child(…)` then use it directly to communicate progress, leaving
       control of the name to the caller. However, call `.init(…)` to configure the iteration.
     * and when calling `add_child(…)` don't use the parent progress instance for anything else.
* **interruption of long-running operations**
  * Use `gix_utils::interrupt::*` to implement cooperative interruption by polling the operation's interrupt flag.
  * The process-global `gix::interrupt::trigger()` flag affects only operations that explicitly poll it. Reset it with
    `gix::interrupt::reset()` only after the affected work has finished, not after an arbitrary delay.
  * **this is a must if…**
    * …temporary resources like files might otherwise be leaked.
  * **this is optional but desirable if…**
    * …there is no leakage otherwise to support user interfaces. They background long-running operations and need them to be cancellable.

* **when to use interior mutability**
  - Optional abstractions can manage caller-owned caches and buffers using **interior mutability only if part of the mutable state has to be returned as borrow**
    or if otherwise it wouldn't be possible to borrowcheck. Or in other words: start without interior mutability and try to do it the standard way, but switch when needed.
  - The preferred way of using it is to start out as upgradable reader, and upgrading to write if needed, keeping contention to a minimum.
  - Shared ownership alone does not require interior mutability; mutation through shared references does. Locally stored caches may still use an API requiring `&mut self`.
  - Types that are not thread-local must be `Sync`, but only if the `gix-parallel/parallel` is enabled due to the usage of `gix_parallel::…` primitives which won't
    be thread-safe without the feature.
* **when to use shared ownership**
  - Use `gix_parallel::OwnShared` for resources shared by thread-local handles. It selects `Arc` or `Rc` according to the `parallel` feature.
    Measure ownership and synchronization overhead in the relevant workload.

## Architecture background

For background on repository-backed platforms and caches, see [this PR](https://github.com/Canop/bacon/pull/98) and
[this discussion](https://github.com/GitoxideLabs/gitoxide/discussions/675).

## Examples, Porcelain CLI and Plumbing CLI - which does what?

### Plumbing vs Porcelain

Both terms are coming from the `git` implementation itself, even though it won't necessarily point out which commands are plumbing and which
are porcelain.
The term *plumbing* refers to lower-level, more rarely used commands that complement porcelain by being invoked by it or by hand for certain use
cases.
The term *porcelain* refers to commands that prioritize an approachable user experience. They are primarily designed for humans,
but may also be used by scripts and agents.

In any case, both types of programs must self-document their capabilities using through the `--help` flag.

From there, we can derive a few rules to adhere to unless there are good reasons not to:

#### Plumbing

* does not show any progress or logging output by default
* if supported and logging is enabled, it will show timestamps in UTC
* tracing is buffered independently of progress and written to stderr with terminal-aware colors after the command finishes
* `corpus` uses the same trace display while persisting full per-run trees independently of the display level
* repository-independent operations accept explicit inputs; repository operations may discover or open a repository

#### Porcelain

* Provides progress information to stderr during long-running operations; allow it to be disabled with `--quiet`.
* If timestamps are shown, they are in localtime.
* Non-progress information goes to stdout.

#### Summary

Here is the hierarchy of programs - each level requires more polish and generally work to be done.
_Examples_ are materialized ideas that others can learn from but that don't quite have the polish (or the potential) to move up to
_plumbing_ or _porcelain_. _Plumbing_ prioritizes composition, whereas _porcelain_ prioritizes an approachable user experience.

* **Examples**
  * An implementation of ideas for actual occasional use and the first step towards getting integrated into Porcelain or Plumbing CLIs.
  * Proper command-line parsing with Clap.
  * Examples must compile; add focused tests where behavior or correctness warrants them. Elaborate test suites are not required.
  * No progress.
  * High quality Rust code along with idiomatic `gitoxide` usage so people can learn from it.
* **Plumbing CLI (like `gix`)**
  * Use Clap for command-line parsing, including small builds.
  * Journey tests only if it's proven valuable
  * Progress can be turned on using the `--verbose` flag, quiet by default.
  * Examples can be turned into plumbing by adding journey tests and optional progress.
* **Porcelain CLI (like `ein`)**
  * Use Clap for command-line parsing for the best quality CLI experience - it's for the user.
  * Journey tests
  * Support for `--quiet`; line progress is shown by default for long-running operations.
  * Verbose by default.
  * Examples can be turned into porcelain by adding journey tests and progress.

# Maintenance Guide

Run `just --list --group 'Maintenance'` to browse maintenance tasks. The `Makefile` retains specific build and fixture-generation targets used by CI.

## Reproducing CI locally

`just ci` runs every selected check that can run on your host. CI uses the same task
catalogue and commands, distributing them across jobs and additional platforms. Use fast
mode while developing, and exhaustive mode to reproduce the checks used after integration:

```bash
just ci                             # fast: affected work, plus mandatory policy checks
just ci --base stable-gix            # compare a stacked PR against its own base branch
just ci --mode exhaustive            # every applicable configuration on this host
just test                           # alias for exhaustive CI
just ci --dry-run                    # inspect commands without executing checks
just ci --stage tests,doctests        # union of stages; overlapping selectors run once
just ci-plan                         # JSON selection, reasons, tasks and job assignments
just ci-plan --all-platforms          # CI's assignments, including other platforms
just ci-plan --platform windows-x86_64 --mode exhaustive
```

Fast mode compares against the merge base with `origin/main` by default, including staged,
unstaged and untracked changes. Fetch your intended base before comparing. Selection includes
owning crates and all reverse workspace dependencies, including optional, dev, build and
platform-specific dependencies. Known root prose files select no Rust work; crate documentation
is a crate input because it can appear in rustdoc. Manifests, lockfiles, fuzzers, shared tests,
CI/build configuration, unclassified paths and a missing base conservatively select exhaustive
coverage. The plan explains each fallback. An unchanged checkout still checks licenses,
dependency policy, advisories, workflow policy, script modes and the MSRV badge.

All PRs request fast mode. Pushes to `main` or `run-ci/**`, and manual workflow dispatch,
request exhaustive mode. Exhaustive checks on landed commits provide feedback for forward
fixes when fast selection misses an undeclared dependency. CI cancels obsolete PR runs but
keeps landed-commit runs. The `Tests pass` gate requires the planner and all required jobs.
Advisory, package-size and WASM failures remain informational, locally and in CI.

### What runs where

Portable checks run locally on Linux, macOS or Windows. CI assigns them once to Linux x86-64
instead of repeating them on every OS. Native tests run on Linux, macOS and Windows in fast
mode; exhaustive CI adds Linux ARM64, Linux 32-bit x86/ARM, and Windows ARM64. Local Intel macOS
is also supported. Only Windows runs the no-Git and 32-bit Windows cases; only Linux x86-64
runs the constrained pure-Rust Docker check. Journey tests require Unix. CI also retains
Windows baseline doctests and MSRV builds. `--all-platforms` shows these exact assignments;
a local run never attempts another host's native checks.

| Stage | Checks |
| --- | --- |
| `self-test` | Lightweight planner/runner tests; CI runs these once in the planning gate |
| `policy`, `advisories`, `audit` | Dependency licenses, bans and sources; advisory reporting; `audit` selects both |
| `housekeeping` | Workflow credentials/gate policy, executable script modes and MSRV badge |
| `lint`, `check` | Formatting and Clippy; exhaustive adds machete and feature compilation checks (`check`) |
| `tests`, `default-tests` | Native nextest, isolated error tests, and supported Windows checks; `tests` also includes declared feature/hash variants |
| `doctests`, `docs` | Cargo doctests and rustdoc, including declared doctest variants |
| `journey` | CLI journeys: affected `small` in fast mode; all four configurations in exhaustive mode |
| `api` | Stable API comparison and ignored API checker integration tests, on Rust 1.99.0 |
| `msrv` | Minimal dependency resolution and both MSRV builds, in an isolated source copy |
| `auditable`, `size` | Auditable build and package-size reporting |
| `wasm` | Affected supported packages/configurations for the three WASM targets |
| `pure-rust` | Linux x86-64 Docker build with constrained C/C++ tooling |

Independent tasks continue after failures; the command fails if any required task failed.
Within a task, prerequisites remain ordered: a failed journey build cannot run a stale CLI.
Missing tools produce visible failures. Use `--stage` to reproduce a subset without installing
unrelated tools. `just unit-tests`, `just doc-tests`, `just check`, `just ci-test` and
`etc/scripts/ci-check-local.sh --fast|--thorough` delegate to the shared runner. The script's
profiles now mean the same fast/exhaustive modes; it no longer cleans fixtures or marks Tix
commits as checked. Tix QA can set its check mark after verifying a clean checkout.

### Local prerequisites and isolation

Install Git, Bash, `just` 1.58.0, `jq`, and a C toolchain, plus platform libraries required by
the selected features (OpenSSL/pkg-config and liblzma where applicable). The shared runner
selects stable Rust for ordinary tasks and Rust 1.99.0 for API tasks. Stable should match CI's
current stable version; Git and system libraries also need compatible versions. MSRV uses the
version declared by `gix` and nightly Cargo for minimal-version resolution:

```bash
rustup toolchain install stable 1.99.0 nightly "$(just msrv)"
rustup component add --toolchain stable clippy rustfmt
rustup target add --toolchain stable wasm32-unknown-unknown wasm32-wasip1 wasm32-wasip2
# Windows x86-64 also needs:
# rustup target add --toolchain stable i686-pc-windows-msvc
cargo install --locked cargo-nextest --version 0.9.122
cargo install --locked cargo-deny --version 0.20.0
cargo install --locked cargo-machete --version 0.9.1
cargo install --locked cargo-semver-checks --version 0.51.0
cargo install --locked cargo-auditable --version 0.7.6
cargo install --locked cargo-diet --version 1.2.7
```

Housekeeping needs [Mike Farah's `yq` 4.49.2](https://github.com/mikefarah/yq/releases/tag/v4.49.2),
not the unrelated Cargo or Python programs of the same name. The pure-Rust check needs Docker;
it uses a disposable Debian container, a read-only source mount, and the persistent Docker
volume `gitoxide-ci-pure-rust` for Rust/download/build caches. Ordinary checks reuse `target`.

The runner sets `CI=true` locally too, including journey assertions and Git-version checks.
It disables snapshot/baseline updates and overrides inherited fixture-selection variables.
Native fast tests use freshly generated fixtures. Exhaustive native tests compile/list once,
then reuse the same nextest binaries for archived and generated fixtures in separate temporary
source copies. Feature/hash variants also run in an isolated copy, sharing compatible fixture
caches within their task and continuing after individual failures. Their fixture caches cannot
reuse a previous local run's extracted archives. Archive changes cause
a failure and retain the changed copy for inspection. Explicit archive-only fixtures keep
their normal semantics.

Source copies include current tracked and visible untracked files, preserving local edits,
deletions, permissions and safe relative symlinks. MSRV resolution changes only its copied
lockfile. The runner never restores, cleans or stages your checkout. Native tests also check
the original source for unexpected changes because compile-time Rust paths may still point
there. Avoid editing while those checks run; concurrent edits are reported as changes too.

### Maintaining CI selection

The `ci` binary in `internal-tools` (`tests/it`) owns planning, the task catalogue, platform
assignment and execution. `.github/workflows/ci.yml` only provisions tools and schedules the
plan. Its gate builds with `--no-default-features`, requiring only Clap, JSON parsing and the
dependency-free `gix-error`; the existing `it` binary retains the default `tools` feature.

```bash
cargo test --locked -p internal-tools --no-default-features --bin ci
just ci-plan > /tmp/ci-plan.json
just ci --plan /tmp/ci-plan.json --dry-run
```

Plans use `cargo metadata --no-deps`, without resolving all external dependencies. Saved plans
preserve selection; regenerate them after changing the checkout. A CI job can be reproduced
with `--plan FILE --job ID` on its matching platform. Stage selection and normal local execution
use the same tasks. Add custom automation under `tests/it`, not to workflow command bodies.

Crate-specific variants live beside their features in `Cargo.toml`:

```toml
[package.metadata.ci]
tests = [
    { args = ["--features", "serde"] },
    { args = ["--features", "sha256"], env = { GIX_TEST_FIXTURE_HASH = "sha256" } },
]
doctests = [{ args = ["--features", "blocking-io"] }]
extra-inputs = ["another-crate/tests/fixtures"]
```

The owner package is implicit. Grouped cases can specify `packages = ["owner", "other"]` and
run when any member is affected. Arguments go directly to Cargo/nextest; the runner supplies
package selection, `--locked`, `--no-fail-fast` and `--doc`. Keep feature combinations separate
when Cargo unification changes dependency features. Declare external crate input paths in
`extra-inputs` (repository-relative files/directories, without globs). Nextest retains the
settings in `.config/nextest.toml`; Cargo runs doctests. Cargo package selection saves compilation
as well as execution, which nextest test filters alone cannot do.

The catalogue preserves 100 distinct package test variants and six doctest variants, plus
native baselines. Three equivalent `gix-hash` variants were combined after comparing their
resolved feature graphs, as were two `gix-zlib` compile checks already covered by its tests. Clippy covers default compilation; rustdoc's former second command
had the same effective documented crate features and was removed. Journey builds remain
adjacent to their runs because later configurations overwrite the same binary paths.

The original [main CI run on 2026-10-31](https://github.com/GitoxideLabs/gitoxide/actions/runs/41186371733)
took 28m23s; Windows ARM fixture regeneration and Linux feature tests dominated. Fast mode avoids
unaffected packages and additional architecture/fixture configurations. Actual savings depend
on the selected dependency closure and cache state; no new wall-clock CI speedup has yet been
measured for this restructuring.

## Creating a release

Use `just release` to publish crates with the repository's release defaults. Additional release-tool arguments can be passed to this recipe.

`just roll-release <cargo-smart-release-args>` runs the release command and then triggers the GitHub binary workflow for the unique `v*` tag at `HEAD`.
To trigger only that workflow, use `just run-release-workflow`. Workflow commands require an authenticated `gh` CLI.

## Git compatibility checks

Git-version detection and fixture support live in [`gix-testtools`](tests/tools/src/lib.rs).
`should_skip_as_git_version_is_smaller_than()` lets tests skip locally when the installed Git is too old, but deliberately
never skips on CI. Check a failing test's required Git version when reproducing CI-only failures.

For fixtures requiring a particular Git version, `scripted_fixture_read_only_with_git_version()` and its writable counterpart
run the fixture script only with a compatible Git. Otherwise they use a matching archive, or return `None` if none is available.
