# Stability

This project follows [semver]. This guide explains which APIs carry compatibility commitments, when crates
must make those commitments, and how changes to _supported APIs_ and _experimental APIs_ affect version numbers.

The guide itself may evolve as requirements change.

## Terminology

- _dependent crate_
  - A crate which depends directly on a crate in this workspace.
- _downstream crate_
  - A crate which directly or indirectly depends on a crate in this workspace.
- _release_
  - Publication of a new crate version to crates.io, including `0.x` versions.
- _major-zero crate_
  - A crate whose major version is zero, for example `0.1.4`. It may introduce _breaking changes_ in minor
    releases. This says nothing about its quality, completeness, or suitability for production use.
- _major-positive crate_
  - A crate whose major version is one or higher and which makes a _supported API_ compatibility commitment.
    It may also provide _experimental APIs_ behind `unstable`.
- _supported API_
  - The public API for which a _major-positive crate_ makes normal semver compatibility commitments.
- _experimental API_
  - Additional public API available through a non-default `unstable` feature, with the more limited
    compatibility commitments described below.
- _breaking change_
  - A change requiring a _dependent crate_ to adapt, for example because code no longer compiles or
    documented behavior changes incompatibly.

## Versioning and public exposure

Compatibility depends on a crate's API commitments, not on the functionality it provides.

### Implementation-only major-zero crates

A crate may remain a _major-zero crate_ indefinitely if it remains an implementation detail and is never
exposed through another crate's public API. Neither production use nor publication on crates.io requires
it to reach 1.0. _Breaking changes_ in a _major-zero crate_ require a minor version bump,
for example from `0.1.4` to `0.2.0`.

Calling a dependency internally does not expose it. A _dependent crate_ can hide a _major-zero crate_ behind
its own API and types, and update or replace that dependency without changing its _supported API_.

### Publicly exposed crates

A crate exposed through another crate's public API must become a _major-positive crate_ and support the exposed
API. Keep its remaining _experimental APIs_ behind its non-default `unstable` feature. Private
implementation code needs neither stabilization nor an `unstable` gate.

Exposure includes re-exports and types reachable through fields, variants, methods, associated types,
bounds, and trait implementations, not only direct function signatures. Exposing a _major-zero crate_ behind
`unstable` still exposes it; gating does not let it remain a _major-zero crate_ indefinitely.
A _supported API_ must not depend on an _experimental API_, even when feature unification makes it public.

The same rule applies to workspace and external dependencies. Existing departures from it are covered by
[temporary migration exceptions](#temporary-migration-exceptions), not a permanent alternative to stabilization.

### Release conventions

A _release_ must use the appropriate version increment and pass the required checks.

Once `gix` is a _major-positive crate_, the naming convention appends the release year and month
as `+YY.MM`, for example `1.2.3+26.11`. This is semver build metadata: it does not affect version
precedence or compatibility requirements.

## Supported and experimental public APIs

A _major-positive crate_ need not stabilize every capability. It commits to a chosen _supported API_ while
allowing additional _experimental APIs_ behind a non-default `unstable` feature.
These rules apply to all _major-positive crates_, including `gix` once it reaches 1.0.

### Version increments

For a _major-positive crate_, the minimum version increment depends on the changes included in the _release_:

| Change | Minimum increment | Example from `1.2.3` |
| --- | --- | --- |
| Backward-compatible fixes or internal changes | Patch | `1.2.4` |
| New public functionality without breaking the _supported API_ | Minor | `1.3.0` |
| _Breaking changes_ confined to _experimental APIs_ | Minor | `1.3.0` |
| _Breaking changes_ to the _supported API_ | Major | `2.0.0` |

Patch releases must preserve both _supported APIs_ and _experimental APIs_. A _release_ containing multiple
kinds of changes takes the strongest required increment. Explicit major bumps and breaking commit intent
retain any stronger release requirement.

**A minor release need not add features.** It may contain only _breaking changes_ behind `unstable` while
leaving the _supported API_ unchanged. _Downstream crates_ using only the _supported API_ can accept that update
normally; those using _experimental APIs_ must review it before upgrading.

### Consuming experimental APIs

_Downstream applications_ using a workspace crate solely as an implementation detail
may opt into its _experimental APIs_, but must restrict updates to the same minor series.
Once `gix` is a _major-positive crate_, for example:

```toml
gix = { version = "~1.2.3", features = ["unstable"] }
```

This admits `1.2.x` updates from `1.2.3` onward but excludes `1.3.0`, protecting the _dependent crate_ from
accidental _breaking changes_ to _experimental APIs_. Plain `"1.2.3"` permits minor updates and lacks that protection.
An exact requirement such as `"=1.2.3"` also protects against those changes, but excludes patch updates too.
This minor-series requirement also applies within this workspace. Each _dependent crate_ must constrain any
dependency whose _experimental APIs_ it uses; pinning the _dependent crate_ itself does not constrain that
dependency's minor series.

_Downstream libraries_ outside this workspace that expose a workspace crate or its types in their _supported API_
should use only that dependency's _supported API_, including internally, rather than enable or forward `unstable`.
This avoids imposing an experimental minor-series pin on the libraries' own _downstream crates_ using those types.
These downstream libraries can accept normal semver-compatible updates:

```toml
gix = { version = "1.2.3" }
```

_Downstream applications_ using those libraries can opt into the dependency's _experimental APIs_ independently.
Those _experimental APIs_ must not become part of a downstream library's _supported API_.

### Gating experimental APIs

`gix_macros::unstable` keeps annotated items available within the crate while exposing them publicly only
with its `unstable` feature enabled. Gate experimental members as well as whole types or modules where needed;
supporting a type does not imply supporting every member it could expose.

Every gate requires `#[gix_macros::unstable(since = "YYYY-MM-DD")]`. Use the item's introduction date
from Git history, following renames when necessary. The date must be a valid calendar date and is
included in experimental documentation to record API provenance.
Usually the date is sufficient; add a nonempty `reason = "..."` only when it explains something beyond the
signature or the fact that the item is experimental.

### Temporary migration exceptions

The workspace is still transitioning to the public-exposure rule. Until `gix` reaches 1.0, existing
manifest-declared exceptions provisionally support selected types from _major-zero crates_. These are
migration allowances, not permission for publicly exposed dependencies to remain _major-zero crates_ forever.
Before releasing `gix` 1.0, make the exposed crates _major-positive crates_ or remove their exposure.
Gate their remaining _experimental APIs_ behind non-default `unstable`.

Each _dependent crate_'s Cargo manifest is the source of truth for its exceptions:

```toml
[package.metadata.api-check]
allowed-unstable-types = ["dependency_crate::BasicType"]
# Reserve whole-crate exceptions for APIs intentionally supported in their entirety.
allowed-unstable-crates = []
```

Use fully qualified Rust type paths without generic arguments, and Rust crate names with underscores.
A type exception applies to that type, not its entire crate. An API does not require the enclosing crate's
`unstable` feature solely because it exposes a declared exception. Other types exposed by supported fields,
variants, methods, associated types, bounds, and trait implementations still need their own assessment.

An exception permits exposure; it does not establish compatibility across breaking dependency releases.
Compatible `0.x` dependency updates remain allowed. _Breaking changes_ to the _supported API_ or exposed type
identity still require a major release when the _dependent crate_ is a _major-positive crate_. Release API checks
must retain those checks even for exempt dependencies.

### API checks

Release planning compares the _supported API_ and the full API separately. Supported comparisons cover the
feature profiles declared in `package.metadata.api-check.profiles`, excluding features listed in
`package.metadata.smart-release.unstable-features`. Full comparisons enable all those experimental features
together on each profile and include _experimental APIs_. The declared features must work together;
a failed check aborts planning rather than establishing compatibility.

Full API compatibility permits patch releases; _breaking changes_ confined to _experimental APIs_ require
at least a minor increment. _Breaking changes_ to a _major-positive crate_'s _supported API_ require a major
increment. Stronger version requirements are preserved.

Standalone `compare` and `boundary` checks cover only the _supported API_.

`etc/scripts/check-api.sh boundary path/to/Cargo.toml` checks these profiles against the manifest's
exceptions. Direct checks inspect the _dependent crate_'s API and the stability of its referenced dependency
types. Set `package.metadata.api-check.recursive = true` to also follow the supported members of reachable
workspace dependency types; this is planned for `gix`. Experimental members are excluded using the
documentation markers emitted by `gix_macros::unstable`, regardless of whether feature unification made them
public. Dependency rustdoc is generated with the features resolved for each profile of the _dependent crate_.

These boundary checks supplement release compatibility checks. Because they inspect supported profiles,
also review _experimental APIs_ for exposed _major-zero crates_: gating alone does not satisfy the public-exposure rule.

## Downstream compatibility testing

CI should build and test major _downstream crates_ against candidate workspace changes before a _release_.
This surfaces unintended source and behavioral breakage early and lets us validate downstream adaptations
for intentional _breaking changes_.

These checks complement workspace tests and API-compatibility checks; they do not relax version-bump requirements.
Where practical, deprecate APIs in compatible releases before removing or changing them so _downstream crates_ can migrate.

## Minimum supported Rust version (MSRV)

Each crate declares its MSRV through `package.rust-version` in its Cargo manifest. The `gix` crate and its
dependency graph are tested at its MSRV by the [`ci.yml` workflow](.github/workflows/ci.yml).

MSRV increases are assessed per crate and are made only when adopting a newer Rust version improves the
quality of that specific crate's code. Such increases are **not** considered _breaking changes_ under this
project's versioning policy. API compatibility, including a patch release's _experimental API_ guarantee,
does not imply an unchanged compiler requirement.

Please let us know if you need a different compiler support policy so we can consider accommodating it.

## Establishing a supported API

Implementation-only _major-zero crates_ need not become _major-positive crates_. Public exposure requires that
transition, but not completion of every feature. Stabilize the API that _downstream crates_ need; the remaining
_experimental API_ can stay behind `unstable`. The `gix-fs` 1.0 API illustrates this approach.

Before exposing a crate as part of a public API:

- Define the _supported API_ and assess it against the needs of _downstream crates_. Resolve blockers in any
  "`<crate-name>` towards 1.0" tracking issue and review its [`crate-status.md`](crate-status.md) checklist.
  Explicitly defer work outside that scope instead of requiring every planned feature to be complete.
- Ensure the crate and its publicly exposed dependencies are _major-positive crates_. Keep implementation-only
  dependencies private; they may remain _major-zero crates_. Resolve existing [migration exceptions](#temporary-migration-exceptions)
  before `gix` 1.0 rather than treating them as permanent support for exposed `0.x` crates.
- Keep _experimental APIs_ behind a non-default `unstable` feature with dated `gix_macros::unstable` gates.
  Review types and their members, bounds, and implementations. _Supported APIs_ must not require _experimental APIs_
  even if dependency feature unification makes them public.
- Declare `package.metadata.api-check.profiles` and pass their boundary checks. Enable recursive checking
  where transitive commitments are intended, and review the dependency exposure of _experimental APIs_ separately.
- Configure `package.metadata.smart-release.api-check` and, when applicable,
  `package.metadata.smart-release.unstable-features` so release planning checks the _supported API_ and the full API.
  Constrain dependencies providing _experimental APIs_ to a minor series, and run the relevant tests, builds,
  documentation checks, and CI checks of _downstream crates_.

A first stable release has no historical stable baseline; passing boundary checks does not establish
compatibility with earlier releases of _major-zero crates_. Establishing a _supported API_ does not require
stabilizing implementation-only dependencies or all _dependent crates_.

[semver]: https://semver.org
[collaboration guide]: https://github.com/GitoxideLabs/gitoxide/blob/main/COLLABORATING.md
