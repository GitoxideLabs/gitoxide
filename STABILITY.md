# Stability

Even though this project adheres to [semver], this guide aims to explain how semantic versioning is used exactly and how we aim to provide stability within an
ever-changing codebase.

Please note that this guide isn't stable itself and may be adjusted to fit changes in requirements or new ways are discovered.

## Stable API baselines and `unstable`

`gix`, `gix-lock`, and `gix-tempfile` distinguish their stable API baseline from
experimental APIs with an opt-in `unstable` feature. The feature adds access; it
must not change stable signatures, type identities, auto-trait guarantees, or
behavior. Cargo unifies features across consumers, so stable-only callers must
continue working when another caller enables experimental APIs.

The initial gates themselves are breaking changes for existing consumers. Add
`features = ["unstable"]` to continue using gated APIs. Experimental APIs may
change incompatibly; pin a version when relying on them. Stabilization removes a
gate in a compatible release and records the first stable version in the docs.
Do not move an established stable API behind a gate in a compatible release.

Stability covers all reachable public types and implementations, including input
types, fields, callbacks, associated types, generic bounds and trait conversions.
An unstable dependency used only internally does not require a breaking release
of its consumer. Re-exporting a type does not hide its dependency identity.
Foundational plumbing needs compatibility commitments before `gix` reaches 1.0.
Until then, stable consumers declare temporary type exceptions in their Cargo
manifests, as described below. These exceptions permit useful operations while
their dependencies remain pre-1.0; they do not waive breaking dependency identity
or signature checks. Concrete plumbing causes remain diagnostic
details unless a gated API explicitly documents the supported downcast.

For each public type, decide whether construction and exhaustive matching are
intentional contracts. Private fields already protect struct construction.
`#[non_exhaustive]` permits additions, not incompatible changes to existing
members; variants needing extensible fields require their own attribute.

These baselines prepare for stability; they do not promote `gix` or its
foundational crates to version 1.0.

## Checking a release boundary

`it check-api` uses the shared checker from `cargo-smart-release-core`. Until that crate
is published, the internal tool depends on cargo-smart-release's
`stability-support` Git branch, with the exact revision recorded in `Cargo.lock`.
CI builds the library as part of `it`; it does not need a separate
`cargo-smart-release` binary or checkout.

Run `just check-api` to check local changes against the same stable API
checks as CI. It tests the release checker, compiles isolated stable and experimental
consumers, and recursively audits the supported members of `gix`'s reachable
workspace plumbing types against its manifest exceptions in each supported profile.
It checks the current working tree, including uncommitted changes, and respects
`CARGO_TARGET_DIR`. Use Rust 1.99, which produces the supported rustdoc JSON format
(for example, `RUSTUP_TOOLCHAIN=1.99.0 just check-api`). It also compares against
the newest reachable `gix-v*` release tag whose manifest declares an API baseline.
CI fetches full history and tags for this comparison. Until the first supported
release exists, the command reports that there is no release baseline and still
runs the boundary checks. Ungated releases are skipped because introducing the
initial gates is intentionally breaking.

Run `just check-api HEAD` to compare uncommitted changes against the current commit,
or replace `HEAD` with any intended baseline revision. Explicit revisions are always
compared, including ungated predecessors. The command also runs baseline/current
regression fixtures for the release guard, so it requires `cargo-semver-checks` as
described below.

The `gix`, `gix-lock` and `gix-tempfile` manifests opt into the API checker through
`package.metadata.smart-release`. A version of `cargo smart-release` with API
checker support invokes `etc/scripts/check-api.sh` before release mutations.
The normal release command remains the entry point. Crates without this metadata
retain conservative dependency-based version bumps.

This is useful before 1.0: a supported `gix` API break requires the next minor
release, while a verified private dependency update can remain a patch release.
Release planning compares against the latest published release, including an
ungated predecessor; the first release introducing the gates is intentionally
breaking. Unlike automatic local checks, release planning does not skip that baseline.

The checker compares each manifest's explicit stable feature profile against the
previous release, excluding `unstable`. It combines `cargo-semver-checks`, type
signature comparisons, and an audit of reachable dependency identities. A
planned incompatible update of a public dependency remains breaking even when
its types have the same spelling. Private dependency updates can stop propagating
through a verified compatible boundary. Tool failures abort the release plan;
they never count as compatibility.

With `api-check.recursive = true`, release mode collects the same supported graph
as the boundary audit in both revisions. It compares the supported signatures and
dependency identities throughout that graph, and runs `cargo-semver-checks` on
each workspace dependency's supported subset. This protects exempt plumbing types'
fields, methods, bounds, associated types, trait implementations and auto traits.
Experimental members and unrelated dependency APIs are excluded. Removing an
exception, marking an existing member experimental, disabling recursion or dropping
a supported feature profile cannot erase a promise made by the baseline. New
profiles are audited before becoming part of the next baseline.

To run the shared API check, including uncommitted changes, without the regression
tests and downstream compile probes, use:

```sh
bash etc/scripts/check-api-compare.sh HEAD
```

The script delegates to `it check-api compare gix --baseline HEAD`, using the same
baseline selection and checks as `cargo api-check gix`. Omit `HEAD` to select the
latest supported release automatically, or audit the current boundary if none exists.
Use `it check-api compare <crate>` directly to check another workspace crate.
The local command checks the versions currently in the manifests; the release
planner also supplies planned breaking dependency updates. The script shares the
existing Cargo target directory with the archived baseline, honoring
`CARGO_TARGET_DIR` when set.

Install `cargo-semver-checks` 0.51.0 or newer on `PATH` (for example,
`cargo install cargo-semver-checks --version 0.51.0 --locked`). The checker relies
on its distinct exit codes for incompatible APIs and operational failures.
Rustdoc JSON generation uses the installed Rust toolchain with `RUSTC_BOOTSTRAP=1`
only for the unstable JSON output option. The collector currently requires format
61, tested with Rust 1.99; other formats and unknown item shapes fail the check.
The toolchain's JSON format and the checker version must be compatible.
Compatibility checks complement behavior tests and the MSRV and platform matrix.
They are not a proof of every semantic compatibility rule.

Capture the initial baseline after gating. Subsequently compare against the
latest release so later stabilizations also become protected. The first gated
release will correctly differ incompatibly from its ungated predecessor.
`etc/scripts/check-gix-api.sh` separately checks downstream access with the feature
both disabled and enabled, outside workspace feature unification. Boundary audits
apply the consuming manifest's exceptions to workspace plumbing; other dependencies
remain subject to release signature and identity checks.

## Terminology

- _dependent crate_
  - A crate which depends on a crate in this workspace directly.
- _downstream crate_
  - A crate which directly or indirectly depends on a crate in this workspace.
- _workspace crate_
  - A crate which is a member of this workspace and hence is stored in this repository
- _breaking change_
  - A change in code that requires a _dependent crate_ to adjust their code to fix compile errors.
- _release_
  - A new version of a crate is published to crates.io
- _development version_
  - A crate version whose _major_ version is 0.
- _release version_
  - A crate version whose _major_ version is 1 or higher.
- _initial development phase_ (IDP)
  - The phase of development leading up to producing a crate with a major version of 1 or greater, as per [semver].
  - Not to be confused with the term _pre-release_, which is used to indicate any release version prior to an actual release, like `1.1.0-beta.1`.

## Tiers

The project uses three stability tiers for all of its crates, and all crates use [semver] for their version numbers.

Tiers differ primarily in the time between breaking changes, which always have to be announced with *PRs* as per
our [collaboration guide].

The following schematic helps to visualize what follows.

```text
                Release Software v1.X
    Stability Tier 1 ═════════════════════════════╗
    ║                                             ║
    ║    gix──────────────┐ ein──────────────┐    ║
    ║    │  plumbing app  │ │  porcelain app │    ║
    ║    └────────────────┘ └────────────────┘    ║
    ║             │                  │            ║
    ║             ▼                  ▼            ║
    ║    gitoxide-core───────────────────────┐    ║
    ║    │        application functionality  │    ║
    ║    └───────────────────────────────────┘    ║
    ║                      │                      ║
    ║                      ▼                      ║
    ║    gix   ──────────────────────────────┐    ║
    ║    │                application crate  │─ ─ ╬ ─
    ║    └───────────────────────────────────┘    ║  │
    ║                      │                      ║
    ║                      ▼                      ║  │
    ║    Foundation Crates───────────────────┐    ║
    ║    │ ┌─────────────┐   ┌─────────────┐ │    ║  │
    ║    │ │  gix-hash   │   │  gix-actor  │ │    ║
    ║    │ └─────────────┘   └─────────────┘ │    ║  │
    ║    │ ┌─────────────┐   ┌─────────────┐ │    ║
    ║    │ │   gix-ref   │   │ gix-config  │ │    ║  │
    ║    │ └─────────────┘   └─────────────┘ │    ║
    ║    │ ┌─────────────┐   ┌─────────────┐ │    ║  │
    ║    │ │ gix-object  │   │  gix-lock   │ │    ║
    ║    │ └─────────────┘   └─────────────┘ │    ║  │
    ║    │ ┌───────────────────────────────┐ │    ║
    ║    │ │         gix-parallel          │ │    ║  │
    ║    │ └───────────────────────────────┘ │    ║
    ║    └───────────────────────────────────┘    ║  │
    ║                                             ║
    ╚═════════════════════════════════════════════╝  │

    Stability Tier 2 ─────────────────────────────┐  │
    │                                             │
    │    Plumbing Crates─────────────────────┐    │  │
    │    │ ┌─────────────┐   ┌─────────────┐ │    │
    │    │ │   gix-odb   │   │  gix-diff   │ │    │  │
    │    │ └─────────────┘   └─────────────┘ │    │
    │    │ ┌─────────────┐   ┌─────────────┐ │    │  │
    │    │ │gix-traverse │   │  gix-pack   │ │◀ ─ ┼ ─
    │    │ └─────────────┘   └─────────────┘ │    │
    │    │ ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐ │    │
    │    │            …many more…            │    │
    │    │ └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘ │    │
    │    └───────────────────────────────────┘    │
    │                                             │
    └─────────────────────────────────────────────┘
```

### Tier 3: IDP crates

Crates in initial development are marked with the major version number zero, for example `0.1.4` and live in stability tier 3 _(->ST3)_.

It's acceptable to let each breaking change be immediately followed by a minor version release.

### Tier 2: released plumbing crates

Released plumbing crates are marked with major version number 1 or above, for example `1.2.4` and live in stability tier 2 _(->ST2)_.

*These crates may not expose parts of unstable crates in their supported public API*.

Until `gix` reaches 1.0, a stable crate may provisionally support selected types from
`0.x` plumbing crates. Its Cargo manifest is the source of truth for these exceptions:

```toml
[package.metadata.api-check]
allowed-unstable-types = ["experimental_plumbing::BasicType"]
# Reserve whole-crate exceptions for APIs intentionally supported in their entirety.
allowed-unstable-crates = []
```

Use fully qualified Rust type paths, without generic arguments, and Rust crate names
with underscores. An exception applies to the named type, not its entire crate. An API
does not require the enclosing crate's `unstable` feature solely because it exposes
a declared exception. Other types exposed by supported fields, variants, methods,
associated types, bounds, and trait implementations still need their own assessment.

This exception permits exposure; it does not establish compatibility across breaking
dependency releases. Compatible `0.x` dependency updates remain allowed. Incompatible
updates that change a supported public API or exposed type identity still require
a breaking release of the affected crate; release API checks must retain those checks.
Recursive release checks protect the supported subset even while its defining crate
remains `0.x`; changes to that subset must respect the consuming crate's release
contract. Establish these commitments before releasing `gix` 1.0.

A crate may offer additional experimental APIs behind a non-default `unstable` feature. These APIs are excluded
from its compatibility guarantee; consumers using them must pin an exact version. `gix_macros::unstable` keeps
annotated items available within the crate while exposing them publicly only with this feature enabled.
Every gate requires `#[gix_macros::unstable(since = "YYYY-MM-DD")]`. Use the item's introduction date
from Git history, following renames when necessary. The date must be a valid calendar date and is
included in experimental documentation. Usually the date is sufficient; add a nonempty `reason = "..."`
only when it explains something beyond the signature or the fact that the item is not stabilized.

`gix-fs` limits its stable API to the operations needed by `gix-tempfile` and `gix-lock`: `dir::create::Retries`
(including its public fields), `dir::create::all()`, `dir::remove::empty_upward_until_boundary()`, and
`set_shared_repository_permissions()`. Other filesystem APIs remain experimental, including the creation/removal
iterators, capability probing, file and symlink helpers, directory traversal, stacks, and snapshots.
Its `serde` and `walkdir` features also enable `unstable` and are excluded from stable release checks.
Stable release checks cover the supported feature profiles declared in `package.metadata.api-check.profiles`, excluding
features listed in `package.metadata.smart-release.unstable-features`.

`check-api boundary path/to/Cargo.toml` checks these profiles against the manifest's
exceptions. Direct checks inspect the consuming crate's API and the stability of its
referenced dependency types. Set `package.metadata.api-check.recursive = true` to
also follow the supported members of reachable plumbing types; this is enabled for
the porcelain crate. Experimental members are excluded using the documentation
markers emitted by `gix_macros::unstable`, regardless of whether feature unification
made them public. Dependency rustdoc is generated with the features resolved for each
consumer profile. Release comparisons reuse this same graph for baseline and current
APIs, rather than maintaining a separate list of protected plumbing types.

Breaking changes are collected and may be released no more often than every 4 weeks by incrementing the major version number.

For example, `gix-odb` and `gix-ref` both have breaking changes, where `gix-odb`'s change is on August 1st and `gix-ref`'s dependent change
is on September 10th. `gix-odb`'s breaking change can be released earliest on September 1st, whereas `gix-ref` can be released earliest at October 10th.

If there are additional breaking changes without a release, these push back the earliest release date accordingly.

### Tier 1: released apps and application crates

Released apps and application crates are marked with major version number 1 or above, like `2.3.0+21.06` and live in tier 1 _(->ST1)_,
with the build identifiers for year (`21`) and month (`06`) appended, based on the actual release year and month.

Breaking changes are collected and may be released no more often than every 6 months by incrementing the major version number. If there are additional breaking changes,
these push back the release date so that they can be tested at least for 3 months. For example, a breaking change happens in January 01, and another breaking change in February 15.
The earliest release date is July 1st. Had the second breaking change happened in April 01, the release date would have to be pushed to August 1st.

Intermediate pre-releases may be created at most every 4 weeks by appending `-alpha.X` where `X` is the sequential release number. These should help testing
breaking changes or new features without forcing the use of `git` sources for dependencies in cargo manifests. Pre-release releases must pin all the pre-release
crates they depend on to prevent automatic upgrades. Dependent external crates are advised to pin their `alpha` dependencies with `=<version>` version requirements to avoid
automatic updates which may be breaking.

Once breaking changes are known to be planned, deprecation warnings should be provided in intermediate pre-releases.

Minor version updates for new features can be released when needed assuming there are no other breaking changes, updating the build identifiers for year and month accordingly.

## The _Minimal Supported Rust Version_ (->MSRV)

Each crate declares its own MSRV in its manifest, and increases it if there is relevant code improvements coming iwth it.

The MSRV is automatically assumed to be the latest stable version for all crates with the following exception: `gix` and all it's dependencies must
adhere to an MSRV, as validated by the `ci.yml` GitHub workflow.

Increasing the MSRV is *not* considered a breaking change like is the case for most other crates in the community.

Please let us know if you have other requirement and we see if we can provide stability guarantees for it or reduce the MSRV to a given version.

## Transitioning from IDP to production crates

How do we avoid staying in the initial development phase (IDP) forever?

There is a couple of questions to ask and answer positively:

- _Is everything in it's tracking issue named "`<crate-name>` towards 1.0" resolved?_
- _Are all checkboxes in its section in `crate-status.md` ticked (and is it OK if not)?_
- _Does the crate fulfill its intended purpose well enough?_
- _Do the dependent workspace crates fulfill their intended purposes well enough?_
- _Do they hide types and functionality of lower-tier workspace crates and external IDP crates?_

For plumbing crates, the intended purpose is narrow which would allow them to transition earlier. For plumbing crates, if in doubt or fear of future requirements
especially if dependent crates are still early in development, prefer to release them anyway and live with requirements of _ST2_.

Apps and application crates may take longer as they are larger in scope. A good indicator for them to get to a release may be maturing plumbing crates they
use. Their scope should also be narrowed to a minimal viable product.

[semver]: https://semver.org
[collaboration guide]: https://github.com/GitoxideLabs/gitoxide/blob/main/COLLABORATING.md
