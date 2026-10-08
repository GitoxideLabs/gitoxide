# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### New Features

 - <csr-id-1f56236675ef84daa5b5acbab0a1555efec9f593/> make caller-location printing opt-in
   <!-- agent -->
   `auto-chain-error` also printed source locations in user-facing reports,
   including consumers which only wanted a standard error chain. Make printing
   independently configurable with `error-print-location`, forwarded by `gix`.
   The feature gates the shared location writer; caller locations, metadata,
   classification, and the existing alternate-format rules remain intact.
   
   Enable printing explicitly in workspace binary dependencies and the filter
   test-helper feature. `gix-error` opts in for its own tests so existing location
   snapshots retain their coverage. An isolated Cargo consumer checks real
   `main()` stderr for both crates, printing on and off, tree and chain modes,
   and `tree-error` precedence without workspace test feature unification.

### Bug Fixes

 - <csr-id-a347e617e7bd1304e9518a9a182e19194b833d47/> shorten caller locations to package source paths
   <!-- Byron -->
   
   Reviewed and refactored carefully, tried the release script as well.
   Didn't review the release script at all though, just had some docs added.
   I consider it disposable.
   
   <!-- agent -->
   Enabled location diagnostics could expose entire checkout and Cargo registry
   paths, obscuring the useful source location with machine-specific directories.
   Format conventional source paths relative to their containing package directory
   and remove Cargo registry version suffixes, so a path such as
   `/home/user/.cargo/registry/src/index.crates.io-hash/gix-url-0.39.0/src/parse.rs`
   prints as `gix-url/src/parse.rs`. Unrecognized layouts fall back to the filename.
   
   Keep captured `std::panic::Location` values and line numbers intact. The shared
   formatter covers tree, chain, source, test, and `main()` diagnostics; feature
   defaults and alternate formatting remain unchanged. Document the limitation
   that locations contain no Cargo package metadata and build owners can use
   `--remap-path-prefix` for exact package names in arbitrary layouts.
 - <csr-id-d18a82b1cafa15d01d02cabefff1cf580f47902b/> preserve explicit causes in standard error source chains
   <!-- agent -->
   Returning a child frame's bare diagnostic from `Error::source()` lost any
   explicit causes attached to that frame. Standard error reporters, including
   `anyhow`, could truncate nested context chains before reaching the underlying
   error.
   
   Keep nonleaf frames reachable as source boundaries while exposing raw leaf
   diagnostics for downcasting. Format each boundary as a single diagnostic to
   avoid duplicating subtrees in external reports, and preserve native-source
   precedence without another allocation.
   
   Add regression coverage for standard and `anyhow` traversal and rendering,
   erasure and boxing round trips, and preservation of typed diagnostics,
   classification, and metadata. Document linear source traversal and the
   full-tree inspection alternatives.
 - <csr-id-23321850e43c434cebd4eeaba40462af0033c178/> flatten native error chains in linear time
   <!-- agent -->
   `Exn::into_chain()` resolved each native source from its owning root both
   to check for nested boundaries and to construct the next handle. Flattening
   64 errors therefore required 4,096 `source()` calls.
   
   Scan each native chain once when constructing its `ErrorHandle`, recording
   its length and the prefix that inherits the frame location. Advancing a
   handle only increments its depth. Stop at nested `Error` boundaries to
   preserve breadth-first order, concrete error types, and caller locations
   without extra allocations or dependencies.
   
   Add `native_source_flattening_is_linear` to require at most four source
   lookups per error and separately verify that every error survives.
   Chains of 16, 32, and 64 errors now need 16, 32, and 64 source calls.
   Explicit `into_chain()` calls exercise flattening in every feature
   configuration. Individual handle lookups continue to walk from their owner.

### Changed (BREAKING)

 - <csr-id-aa7b69919317fdef6882895deb03988699c72859/> wrap `Metadata` and simplify diagnostic keys
   <!-- agent -->
   Quoting every metadata key adds noise to diagnostic output even when the
   name is already unambiguous. Keep nonempty keys containing only ASCII
   letters, digits, `_`, `-`, or `.` unquoted in `Message` display and metadata
   debug output. Empty and unusual keys remain quoted and escaped, and value
   formatting is unchanged.
   
   Replace the public `BTreeMap` alias with a private-storage `Metadata`
   wrapper so dictionary formatting and its public API no longer expose the
   implementation. Preserve lexicographic ordering and provide construction,
   lookup, mutation, iteration, indexing, collection, and extension APIs.
   
   **Breaking change:** `Metadata` is no longer interchangeable with
   `BTreeMap`. Callers must use the wrapper API; iterator return types hide
   the underlying collection.
   
   Add regression coverage for key quoting, compact values, and dictionary
   access, and update dependent assertions and output snapshots.
 - <csr-id-506fc1df9ad7931139503a80d3a51aa0117a0ef8/> unify workspace diagnostics and error recovery
   <!-- Byron -->
   
   Looked at `gix-error` carefully and did multiple cleanup and improvement passes.
   At some point I went quicker through the changes as they looked good enough,
   also to at some point just wave it through.
   
   <!-- agent -->
   ## General changes
   
   Use the workspace's own error API throughout applications, examples, internal
   utilities, and fuzz targets. Replace downstream `anyhow::Result`, `Context`,
   `bail!`, and `ensure!` usage with `gix::Result` and helpers from `gix::error`,
   or `gix_error` in plumbing crates. Convert native errors at their call sites
   while preserving concrete causes, typed recovery contracts, and caller locations.
   Keep optional `anyhow` interoperability in `gix-error` and `gix` for consumers.
   
   Make diagnostics useful both to people and to callers deciding how to recover.
   Distinguish invalid input, corrupt data, missing resources, unavailable
   capabilities, authentication, authorization, conflicting state, and cancellation.
   Explicit cancellation prevents retrying even when another cause is retryable.
   Store offending input and available subprocess details as structured metadata,
   with operation context in the message and native failures retained as causes.
   
   Report causes under a numbered `Caused by:` section, retaining branch guides for
   error trees and flattening linear chains. Standardize path quoting, refresh
   snapshots, and omit changing source locations from CLI error comparisons.
   Migrate affected tests and helpers to `TestResult` where they need to propagate
   both native errors and exceptions. Update contributor and application guidance,
   remove redundant dependencies, and refresh the lockfile.
   
   ## `gix-error`: recovery classes, metadata, and complete native reports
   
   Add `Cancelled`, `PermissionDenied`, `Unauthenticated`, `Conflict`, and
   `Unsupported` classes with matching constructors, markers, builders, and
   predicates. Order classes by suggested recovery precedence and expose
   `dominant_class()`. Both retry policies inspect cancellation across causes;
   native permission and unsupported I/O errors receive classifications, while
   explicit authentication challenges take precedence over legacy permission wrappers.
   
   Add classified `Message` builders and `_error` helpers, caller-tracked
   `Message` conversions, and formatted `bail!`/`ensure!` shorthand with builder
   chains. Introduce `with_input()`, subprocess metadata helpers, common metadata
   schemas, and `metadata_merged()`, with more specific causes overriding context.
   
   Number tree reports while preserving forks and flattening chains. Give
   `auto-chain-error` complete `Debug` reports with locations and single-line
   alternate `Display` chains; share that formatter with `TestError`. Cover nested
   native sources, classification precedence, metadata, caller tracking, macro
   evaluation, report layout, and retained `anyhow` interoperability.
   
   ## `gix`: consistent porcelain diagnostics and application guidance
   
   Document `gix::Result`, `gix::Error`, and `gix::error` for applications. Apply
   classification and input metadata to configuration, initialization, repository
   opening, clone and remote setup, object access, and path handling. Distinguish
   unsupported formats from malformed values, and classify branch/worktree state
   conflicts and explicit cancellation without replacing concrete recovery errors.
   
   Record signing and key-command failures through subprocess metadata. Migrate
   examples and tests away from `anyhow`, refresh diagnostics and classification
   assertions, remove the test dependency, and update `prodash` to `31.0.1`.
   
   ## `gitoxide`: native CLI errors and dependency cleanup
   
   Return `gix::Result` from `gix`, `ein`, the combined binary, and shared command,
   progress, and tracing helpers. Preserve context when configuring repositories,
   writing output, and starting tracing; classify unavailable tracing as unsupported.
   Migrate top-level examples and journey snapshots to native diagnostics.
   Replace `is-terminal` with `std::io::IsTerminal`, remove `anyhow`, and update
   `prodash` to `31.0.1` with corresponding feature and lockfile cleanup.
 - <csr-id-7cbd9eb87e822db146c09c4403851330886d71ef/> return public errors from extension helpers
   <!-- Byron -->
   
   `gix-error` was refackiewed, the rest was scrolled through to peek out problems
   at best.
   Looked at `gix` in detail, refackiew, and came up with API improvement to
   `gix-config-value` on the way.
   
   <!-- agent -->
   Public plumbing APIs now use `gix_error::Result`, so constructing typed
   exceptions by default requires repeated erasure, conversion, and propagation
   wrappers at their call sites.
   
   Make `raise()` and `and_raise()` return `Error`, and make `or_raise()` and
   `ok_or_raise()` return `Result`. Preserve the former behavior under explicit
   `_typed` names and add `ResultExt::or_error()` for conversion without context.
   Existing public errors pass through unchanged without allocating, while
   context retains concrete causes, metadata, classifications, and caller sites.
   Keep the erased helpers and inherent `Exn` operations unchanged.
   
   Migrate callers and documentation together, preserving concrete public
   exception signatures and typed internal boundaries. Use the existing `bail!`
   macro for 388 early error returns across 37 crates, passing native errors and
   messages directly and retaining context and explicit erasure on typed
   exceptions. Preserve direct propagation of public `Error` values to avoid
   introducing extra exception frames.
 - <csr-id-ac71fb42cd2417d27dfd0c9f4ec769abd1e95839/> standardize public errors and reuse exception frames
   <!-- Byron -->
   rubber-stamp after scrolling through the diff top to bottom.
   gix-error changes were refackiewed carefully.
   
   <!-- agent -->
   Use `gix_error::Result` and `gix_error::Error` at public plumbing boundaries
   with erased or message-based exceptions so applications can use ordinary
   `std::error::Error` handling while retaining concrete recovery errors,
   causes, metadata, and caller locations. Apply this to public traits,
   callbacks, iterator items, and re-exported APIs, and adapt their consumers
   throughout the workspace. Preserve concrete public error signatures and
   typed internal exceptions; let `bail!` convert to either result form.
   
   Add `Error::into_exn()` for internal tree manipulation, including the
   revision parser's delayed errors. Recover explicitly raised frames in
   `auto-chain-error` mode and forward native source chains through nested
   `Error` wrappers. Reuse existing boxed tree frames when adding context or
   erasing errors to avoid redundant boxing, and keep context construction
   independent of existing chain depth.
   
   Remove repeated propagation wrappers from callers and update the migration
   guide and repository conventions. Document the value of context that
   explains an operation's intent or identifies user-controlled configuration,
   including its raise location, even when failures are rare.
   
   Remove `Class::Io` and the synthetic `io()` constructor so classifications
   describe semantic failures. Retain `NotFound` and `OutOfMemory`
   classifications for real I/O errors, and inspect their concrete causes for
   the conservative and lenient retry policies.
 - <csr-id-a36bff1d40022d35820cbea693bbad093e631464/> replace recovery tags with typed operation errors
   <!-- agent -->
   GitButler needs to distinguish missing binary merge results, ambiguous object
   prefixes, diff setup failures, unrelated histories, and repository discovery
   outcomes. Broad classifications alone also match failures that these callers
   must propagate, such as missing-directory I/O and rejected repository trust.
   
   Remove `Class::Tagged` and expose non-exhaustive `Error` enums for these
   operations. Share the merge-base enum across pair, many, and octopus methods,
   and keep porcelain returns as `gix::Result`. Preserve candidate information,
   native paths, diff resource details, requested commit IDs, and callee causes.
   
   Add `gix_error::tag()` and make classifications identify the wrapped error or
   the owner of a constant marker source. Keep traversal lazy and preserve its
   order, duplicates, marker transparency, and original I/O kinds in both error
   representations. Document typed recovery and wildcard propagation.
   
   Validate merge-base input commits in the shared traversal path so missing
   inputs cannot masquerade as unrelated histories, including fast paths and
   octopus calls. Continue allowing absent parents for shallow histories and
   empty successful results for `merge_bases_many*()` on unrelated histories.
   
   Fix the recovery tests failing in Linux/ARM `test-fast` and Windows
   `test-fixtures-windows` CI. Discovery examines the ceiling directory before
   stopping, so its immediate child reports a stopping height of 2. Attribute
   lookup converts Git paths to native paths, rejecting ill-formed UTF-8 on
   Windows before reaching object lookup. Cover missing-object conversion with
   a valid path everywhere and retain its non-UTF-8 path case on Unix.

### New Features (BREAKING)

 - <csr-id-f0940c590e7142820ac191198f4539b31e48bc42/> make `bstr` an opt-in metadata conversion feature
   <!-- agent -->
   Small error-handling programs should not compile `bstr` and `memchr` just
   to add ordinary error contexts. Disable `bstr` by default so a standalone
   `gix-error` consumer needs no dependencies unless it enables optional features.
   
   Always store `MetadataValue::Bytes` as `Vec<u8>`, regardless of features.
   This changes the public variant payload from `BString`. Keep escaped debug
   and display diagnostics compatible with `bstr`, including valid Unicode,
   control characters, invalid bytes, and truncated UTF-8.
   
   The opt-in `bstr` feature supplies `From<BString>` and `From<&BStr>` for
   `MetadataValue` and retains the crate re-export. Owned conversions reuse
   the allocation; borrowed conversions copy the bytes. Enable the feature
   explicitly in workspace crates that need these conversions or the re-export,
   preserving their `with_input()` and `with()` calls. Adapt direct byte
   constructors and submodule payload access to the new `Vec<u8>` representation.
   
   Earlier five-sample fresh-build measurements of the same small consumer on
   Rust 1.99.0 / M4 Max recorded medians of 702 vs 458 ms in debug and 872 vs
   609 ms in release with and without `bstr`. These include Cargo and linking
   with registry sources cached; they are specific to that consumer and machine.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 15 commits contributed to the release over the course of 12 calendar days.
 - 13 days passed between releases.
 - 10 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge pull request #3032 from GitoxideLabs/sec-audit ([`1d7bac7`](https://github.com/GitoxideLabs/gitoxide/commit/1d7bac742f70b72ddda0c7294f4a97566b8db596))
    - Shorten caller locations to package source paths ([`a347e61`](https://github.com/GitoxideLabs/gitoxide/commit/a347e617e7bd1304e9518a9a182e19194b833d47))
    - Make caller-location printing opt-in ([`1f56236`](https://github.com/GitoxideLabs/gitoxide/commit/1f56236675ef84daa5b5acbab0a1555efec9f593))
    - Preserve explicit causes in standard error source chains ([`d18a82b`](https://github.com/GitoxideLabs/gitoxide/commit/d18a82b1cafa15d01d02cabefff1cf580f47902b))
    - Wrap `Metadata` and simplify diagnostic keys ([`aa7b699`](https://github.com/GitoxideLabs/gitoxide/commit/aa7b69919317fdef6882895deb03988699c72859))
    - Merge pull request #3046 from GitoxideLabs/gix-error-optional-bstr ([`9272d45`](https://github.com/GitoxideLabs/gitoxide/commit/9272d45a1b08ca6f26779389954bd718f8308a80))
    - Make `bstr` an opt-in metadata conversion feature ([`f0940c5`](https://github.com/GitoxideLabs/gitoxide/commit/f0940c590e7142820ac191198f4539b31e48bc42))
    - Merge pull request #3033 from GitoxideLabs/gix-cli-progress-cleanup ([`80f4b03`](https://github.com/GitoxideLabs/gitoxide/commit/80f4b03257da9a664468e7b75247023187565a34))
    - Unify workspace diagnostics and error recovery ([`506fc1d`](https://github.com/GitoxideLabs/gitoxide/commit/506fc1df9ad7931139503a80d3a51aa0117a0ef8))
    - Merge pull request #3022 from GitoxideLabs/release-testtools ([`f819565`](https://github.com/GitoxideLabs/gitoxide/commit/f819565c2c4c56619c4888acef6cf3b8144cbccb))
    - Flatten native error chains in linear time ([`2332185`](https://github.com/GitoxideLabs/gitoxide/commit/23321850e43c434cebd4eeaba40462af0033c178))
    - Return public errors from extension helpers ([`7cbd9eb`](https://github.com/GitoxideLabs/gitoxide/commit/7cbd9eb87e822db146c09c4403851330886d71ef))
    - Standardize public errors and reuse exception frames ([`ac71fb4`](https://github.com/GitoxideLabs/gitoxide/commit/ac71fb42cd2417d27dfd0c9f4ec769abd1e95839))
    - Replace recovery tags with typed operation errors ([`a36bff1`](https://github.com/GitoxideLabs/gitoxide/commit/a36bff1d40022d35820cbea693bbad093e631464))
    - Merge pull request #3020 from GitoxideLabs/report-september ([`5fb3dcf`](https://github.com/GitoxideLabs/gitoxide/commit/5fb3dcf6a86ac0c403776c8820bf5d23f187f7e1))
</details>

## 0.4.0 (2026-09-25)

### New Features

 - <csr-id-dcdddd9429e3faf2083be340f9ec2bfaab232c2f/> attach scalar metadata to error contexts
   <!-- agent -->
   Callers can enrich errors with named values they already possess without
   introducing a custom payload type. `Metadata` keeps a message and an ordered
   dictionary of typed scalar values, including lossless bytes and native paths.
 - <csr-id-82776d93ab7671729b80ad89e420236c93cd1651/> inspect borrowed error classifications lazily
   Admittedly, I was mostly checking the public API, thinking that it's
   most certainly useful and overally, having lazy iterators for everything
   is a step in the right direction.
   
   This wasn't a crazily detailed review in the interest of time.
   
   <!-- agent -->
   Error inspection rebuilt the entire error graph before returning its first
   item, so even a root match visited unrelated sources and allocated storage.
   Use a shared iterator that expands each node only when another item is needed,
   preserving breadth-first order, concrete types, and caller locations in both
   error representations.
   
   Expose `classify(&error)` for custom borrowed errors, and share classification
   predicates with `Error` and `Exn`. The shared traversal and predicate definitions
   remove more code than the borrowed API adds. Keep `probable_cause()` unchanged.
 - <csr-id-4fa58c5aee832a70372ad6c02f8573b8284bf040/> expose borrowed exception retry policies
   <!-- agent -->
   Add `Exn::can_retry()` and `Exn::can_retry_lenient()` using the existing
   classification and retry rules. Callers can now inspect retry policies
   without consuming an exception to produce `Error`.
   
   Cover native sources, nested errors, explicit markers, all recognized I/O
   retry kinds, and allocation failures in both error storage modes.
 - <csr-id-ef8ced5db1c668efaff7485e501f78be447b88d0/> expose borrowed exception classifications
   <!-- agent -->
   Add `Exn::classify()` so callers can inspect normalized classes and their
   original errors without converting the exception into `Error`. Reuse the
   existing collection and classification logic, preserving breadth-first
   ordering through nested boundaries, duplicate classes, and I/O origins.
   Route the exception predicates through that shared classification path.
 - <csr-id-ce38e3b6529b4df52631b02c6116580ddf4d73cd/> simplify borrowed error inspection
   <!-- agent -->
   Expose semantic predicates on `Exn` and `Error` so callers can classify
   failures without downcasts or ownership-consuming conversions. Preserve
   explicit retry markers as distinct from the I/O retry policies.
   
   Make nested error lookup follow logical breadth-first order and add
   borrowed `Exn::probable_cause()`. Distinguish native-source identities even
   when a source shares its owner's address, preserving the selected cause
   through conversion. Reuse the existing inspection collectors.
   
   Consolidate representation-independent helpers and document conversions
   that retain context and caller locations.

### Bug Fixes

 - <csr-id-ac88bd7307b584183815e124b86fa44ac59b3988/> preserve classifications in custom I/O payloads
   <!-- agent -->
   `std::io::Error::source()` skips its custom payload, hiding classification
   markers, custom error types, and branches of a nested `gix_error::Error`.
   Retain the payload while walking native sources so borrowed retry policies,
   exception traversal, and both porcelain error modes see the complete cause.
   
   Document how custom errors expose their immediate cause and classification
   markers. Cover every classification, nested branches, and concrete payload
   downcasts, and update diagnostic snapshots for the retained I/O payload.
 - <csr-id-ca0f05dbefd75977f72485533e140015295f9590/> render nested error causes once
   <!-- agent -->
   Rendering a nested `Error` invoked its complete formatter before expanding
   its causes again, duplicating messages and mixing formatting modes. Render
   the concrete root once and skip that exact root during nested expansion;
   its position is not necessarily first after filtering nested boundaries.
   Apply the selected formatting mode and caller locations to nested causes.
 - <csr-id-c28e49464db05f2b510db13dfc60384846b36cfb/> keep drained exceptions safe to inspect
   <!-- agent -->
   Converting a typed `Frame` into a bare `Exn` left its error box typed while
   `error()`, dereferencing, and extraction expected `Untyped`, causing a
   panic after `drain_children()`. Wrap typed roots during the shared frame
   conversion while retaining existing erasure, sources, and child frames.

### Changed (BREAKING)

 - <csr-id-da0f21d3f48266eb47a23548d5ae4163b78651dc/> unify diagnostics as `Message` and preserve causes
   <!-- Byron -->
   
   I refackiewed gix-error with the usual care, and skimmed through all the
   downstream changes, rubber-stamping it.
   Definitely cleaned up a few bits, like config-key related error handling,
   and error related types.
   
   <!-- agent -->
   
   A single failure could require separate errors for its message, category,
   and diagnostic values. Classification wrappers could obscure the concrete
   cause, while cause-selection heuristics could choose a different sibling
   after adding context or flattening an exception tree. Represent each
   failure with one diagnostic and preserve its actual causal structure.
   
   Make `Message` the shared diagnostic type, with a message, optional `Class`,
   and named `MetadataValue`s. Classified constructors and the `with_class()`
   and `with()` builders attach categories and details without extra error
   layers. Plain messages and string conversions remain unclassified.
   `Metadata` is now the value dictionary; `Exn::metadata()` and
   `Error::metadata()` yield only non-empty dictionaries in traversal order,
   keeping each context separate and preserving bytes, paths, and numeric types.
   
   Introduce `ClassificationMarker` to classify existing concrete errors or
   provide classification-only sources. Classification still inspects these
   markers, while diagnostic traversal, downcasts, cause selection, and
   reports skip them and retain their real descendants. Preserve concrete
   I/O errors and their original kinds for recovery decisions.
   
   Give `Frame`, `Exn`, and `Error` the same `probable_cause()` policy: follow
   the unique causal path to a leaf or the first diagnostic branch, retaining
   the aggregate instead of choosing an arbitrary sibling. Include native
   sources, I/O payloads, nested errors, and explicitly raised children in both
   tree and chain representations. Preserve caller locations through transparent
   markers, avoid duplicate native causes in reports, and honor alternate
   `TestError` formatting. Marker-only errors retain a classification fallback.
   
   Add `ExnResult<T = (), E = exn::Untyped>` and `ExnMessageResult<T = ()>`,
   re-export them from `gix`, and adopt them throughout the workspace. Keep
   callback bounds erased and preserve concrete error types where callers
   need their payloads. Group supporting classification and display types in
   `gix_error::types`, and frame inspection and erasure in `gix_error::exn`.
   
   Migrate constructors, signatures, and recovery checks together. Replace the
   `gix::config::key::Error` family with ordinary diagnostics and central
   `gix::Error` results. Provide `config::key::error()` and `error_with_value()`
   for `key`, optional `environment_override`, and optional `input` metadata.
   Retain parser causes, rejected numeric values, accepted special values,
   and configuration leniency. Collapse artificial error layers elsewhere
   while preserving concrete failures and partial outcomes.
   
   Make complete diagnostics reviewable through inline `insta` snapshots,
   retaining assertions for classifications, retry policies, concrete causes,
   and stored data. Use the shared snapshot redaction helper for unstable
   paths, ports, object IDs, and platform-specific OS errors. Update migration
   guidance, examples, and CLI snapshots, and use static X.509 assertion messages to
   avoid dumping verifier identities and raw output.
 - <csr-id-4b42e0ce80ae934cae4f102f44c392581758608f/> raise MSRV to Rust 1.88
   <!-- agent -->
   The newly published `dua-core` 3.3 release used by linked-worktree removal
   requires Rust 1.88, so raise every workspace crate and the advertised badge
   together.
   
   Keep the MSRV checks buildable by selecting the latest `sysinfo` and `rusqlite`
   release lines that support Rust 1.88.

### New Features (BREAKING)

 - <csr-id-b7ebd86cea6043224316b15abf90890e9214a5f0/> expose structured error classifications
   Marked breaking because `can_retry()` now has a much more conservative default.
   So downstream has to adjust and use `can_retry_lenient()` if this is desired.
   
   <!-- agent -->
   Add ordered, non-exclusive classifications that retain the concrete error
   source. Resource exhaustion now distinguishes configured allocation limits
   from allocation or capacity failures, while existing boolean helpers remain
   compatibility views.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 16 commits contributed to the release over the course of 24 calendar days.
 - 24 days passed between releases.
 - 11 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-error v0.4.0, gix-date v0.17.0, gix-actor v0.43.0, gix-trace v0.2.0, gix-validate v0.12.0, gix-path v0.13.0, gix-utils v0.4.0, gix-quote v0.9.0, gix-command v0.11.0, gix-features v0.50.0, gix-hash v0.27.0, gix-hashtable v0.17.0, gix-fs v0.23.0, gix-tempfile v25.0.0, gix-object v0.65.0, gix-glob v0.28.0, gix-attributes v0.36.0, gix-packetline v0.23.0, gix-filter v0.35.0, gix-chunk v0.9.0, gix-commitgraph v0.40.0, gix-revwalk v0.36.0, gix-traverse v0.62.0, gix-worktree-stream v0.37.0, gix-archive v0.37.0, gix-bitmap v0.5.0, gix-lock v25.0.0, gix-index v0.56.0, gix-config-value v0.20.0, gix-pathspec v0.21.0, gix-ignore v0.23.0, gix-worktree v0.57.0, gix-imara-diff v0.3.0, gix-diff v0.68.0, gix-blame v0.18.0, gix-ref v0.68.0, gix-sec v0.15.0, gix-config v0.61.0, gix-prompt v0.18.0, gix-url v0.39.0, gix-credentials v0.41.0, gix-discover v0.56.0, gix-dir v0.30.0, gix-mailmap v0.35.0, gix-revision v0.50.0, gix-merge v0.21.0, gix-negotiate v0.36.0, gix-note v0.2.0, gix-zlib v0.2.0, gix-pack v0.75.0, gix-odb v0.85.0, gix-macros v0.2.0, gix-refspec v0.46.0, gix-shallow v0.14.0, gix-transport v0.60.0, gix-protocol v0.66.0, gix-status v0.35.0, gix-submodule v0.35.0, gix-worktree-state v0.35.0, gix v0.88.0, gix-fsck v0.26.0, gitoxide-core v0.62.0, gix-tix v0.4.0, gitoxide v0.59.0, safety bump 60 crates ([`37860b3`](https://github.com/GitoxideLabs/gitoxide/commit/37860b34db26096c8187ef55bdf4b76705142733))
    - Merge pull request #2847 from GitoxideLabs/gix-error-completion ([`6356013`](https://github.com/GitoxideLabs/gitoxide/commit/6356013bca0987c6c97ad7ba9d5347271979b51e))
    - Unify diagnostics as `Message` and preserve causes ([`da0f21d`](https://github.com/GitoxideLabs/gitoxide/commit/da0f21d3f48266eb47a23548d5ae4163b78651dc))
    - Attach scalar metadata to error contexts ([`dcdddd9`](https://github.com/GitoxideLabs/gitoxide/commit/dcdddd9429e3faf2083be340f9ec2bfaab232c2f))
    - Inspect borrowed error classifications lazily ([`82776d9`](https://github.com/GitoxideLabs/gitoxide/commit/82776d93ab7671729b80ad89e420236c93cd1651))
    - Preserve classifications in custom I/O payloads ([`ac88bd7`](https://github.com/GitoxideLabs/gitoxide/commit/ac88bd7307b584183815e124b86fa44ac59b3988))
    - Merge pull request #2989 from GitoxideLabs/error-conversion-review ([`4b9ff51`](https://github.com/GitoxideLabs/gitoxide/commit/4b9ff511a49f7963e97a669ca82c6f6e833d8ea2))
    - Expose borrowed exception retry policies ([`4fa58c5`](https://github.com/GitoxideLabs/gitoxide/commit/4fa58c5aee832a70372ad6c02f8573b8284bf040))
    - Expose borrowed exception classifications ([`ef8ced5`](https://github.com/GitoxideLabs/gitoxide/commit/ef8ced5db1c668efaff7485e501f78be447b88d0))
    - Render nested error causes once ([`ca0f05d`](https://github.com/GitoxideLabs/gitoxide/commit/ca0f05dbefd75977f72485533e140015295f9590))
    - Keep drained exceptions safe to inspect ([`c28e494`](https://github.com/GitoxideLabs/gitoxide/commit/c28e49464db05f2b510db13dfc60384846b36cfb))
    - Simplify borrowed error inspection ([`ce38e3b`](https://github.com/GitoxideLabs/gitoxide/commit/ce38e3b6529b4df52631b02c6116580ddf4d73cd))
    - Merge pull request #2949 from GitoxideLabs/error-conversion-review ([`a095334`](https://github.com/GitoxideLabs/gitoxide/commit/a0953348e4d27f59222c1782119d2539a778cd4d))
    - Expose structured error classifications ([`b7ebd86`](https://github.com/GitoxideLabs/gitoxide/commit/b7ebd86cea6043224316b15abf90890e9214a5f0))
    - Raise MSRV to Rust 1.88 ([`4b42e0c`](https://github.com/GitoxideLabs/gitoxide/commit/4b42e0ce80ae934cae4f102f44c392581758608f))
    - Merge pull request #2955 from GitoxideLabs/transport-url-encoding ([`7e35849`](https://github.com/GitoxideLabs/gitoxide/commit/7e35849b36646cff9722f6906a4527d64818a374))
</details>

## 0.3.2 (2026-09-01)

### New Features

 - <csr-id-870c191a6d3e87c8806205e64b4446266c381844/> add ergonomic test errors and string comparisons
   <!-- agent -->
   Test functions only require Debug for their error type, so TestError can accept
   both standard errors and Exn without relying on a boxed error as the public
   result type. Direct, asymmetric comparisons with string slices and owned strings
   keep assertions concise while retaining Display semantics.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 8 calendar days.
 - 9 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-path v0.12.6, gix-error v0.3.2, gix-command v0.10.1, gix-transport v0.59.2 ([`888677a`](https://github.com/GitoxideLabs/gitoxide/commit/888677ad2d63a2e3930a02add2de0b4b667a5581))
    - Merge pull request #2944 from GitoxideLabs/error-conversion-review ([`e3a6fa1`](https://github.com/GitoxideLabs/gitoxide/commit/e3a6fa1516481ec69ab00cddcca081ecdc52b4ca))
    - Add ergonomic test errors and string comparisons ([`870c191`](https://github.com/GitoxideLabs/gitoxide/commit/870c191a6d3e87c8806205e64b4446266c381844))
    - Merge pull request #2932 from GitoxideLabs/fundamental-types-comp ([`6704303`](https://github.com/GitoxideLabs/gitoxide/commit/6704303ed5ef3403b129e2b6cc4a9214432ffd03))
</details>

## 0.3.1 (2026-08-23)

### New Features

 - <csr-id-6b8edbba8ad382d218359baa6641256eee8c4809/> add BoxedResultExt for boxed errors

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 1 day passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-error v0.3.1, gix-hash v0.26.2, gix-object v0.64.1, gix-ref v0.67.1, gix-packetline v0.22.1, gix-pack v0.74.1, gix-testtools v0.20.0 ([`e52fe9d`](https://github.com/GitoxideLabs/gitoxide/commit/e52fe9d03e82437a25bdfb1098e7046ec7e1b558))
    - Add BoxedResultExt for boxed errors ([`6b8edbb`](https://github.com/GitoxideLabs/gitoxide/commit/6b8edbba8ad382d218359baa6641256eee8c4809))
    - Merge pull request #2933 from GitoxideLabs/report-august ([`b8914ff`](https://github.com/GitoxideLabs/gitoxide/commit/b8914ffda5bc8f6ea851aaf1f720140acfe96dbb))
</details>

## 0.3.0 (2026-08-22)

### New Features (BREAKING)

 - <csr-id-1af3e724c305862975bab35a6c9c263abb5d894b/> preserve and classify typed error sources
   <!-- agent -->
   Error classification and probable-cause selection need to inspect the complete
   error graph without turning native sources into strings or behaving differently
   across tree and auto-chain modes.
   
   Add CorruptionError, NotFoundError, and RetryableError together with Error
   classification helpers and support for boxed standard errors. Classification
   follows native source chains and nested gix errors while retaining concrete
   types for downcasting.
   
   Keep native sources owned by their original errors and traverse them lazily
   alongside explicit frames. Add breadth-first error iteration with optional
   captured locations, direct stored-error access, and downcasting across the
   complete graph.
   
   Preserve probable-cause identity and logical parent relationships while
   flattening exception trees into ChainedError. This lets auto-chain mode
   reconstruct the same traversal order without duplicating nested compatibility
   source chains.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 38 calendar days.
 - 38 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Update manifests prior to release ([`ebe9095`](https://github.com/GitoxideLabs/gitoxide/commit/ebe9095f2888d3c12447ea5eed9d0afdb0fd5aeb))
    - Merge pull request #2930 from GitoxideLabs/gix-notes ([`7424676`](https://github.com/GitoxideLabs/gitoxide/commit/7424676f86cd3f5a67c53f8db6baf0803e937d4a))
    - Preserve and classify typed error sources ([`1af3e72`](https://github.com/GitoxideLabs/gitoxide/commit/1af3e724c305862975bab35a6c9c263abb5d894b))
    - Merge pull request #2714 from GitoxideLabs/fix-credentials-parsing ([`cf3053a`](https://github.com/GitoxideLabs/gitoxide/commit/cf3053a3c18e2de788cdaa9f41b5bd343bdc0091))
</details>

## 0.2.5 (2026-07-15)

### Bug Fixes

 - <csr-id-ccd5bd412eeaf3fa9537a021d647f818d4a27833/> keep erased errors visible to source-iteration and downcasting
   Since 499402c941, erasing an Exn wraps its error in the Untyped marker so that
   the typed accessors of Exn<Untyped> keep working. However, the marker also hid
   the original error from everything that walks the error afterwards: frames now
   yielded the marker, which cannot be downcast to the original type, and whose
   empty Error implementation cut off the source chain below it.
   
   This broke gix diff file, whose fallback for treating a revspec as a path on
   disk downcasts the sources() of a failed rev-parse to find the ref-not-found
   error - it would now fail with "couldn't parse revision" instead.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 50 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#2694](https://github.com/GitoxideLabs/gitoxide/issues/2694)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2694](https://github.com/GitoxideLabs/gitoxide/issues/2694)**
    - Keep erased errors visible to source-iteration and downcasting ([`ccd5bd4`](https://github.com/GitoxideLabs/gitoxide/commit/ccd5bd412eeaf3fa9537a021d647f818d4a27833))
 * **Uncategorized**
    - Release gix-path v0.12.2, gix-error v0.2.5, gix-utils v0.3.4, gix-date v0.15.6, gix-url v0.36.2, gix-credentials v0.38.2 ([`27aec47`](https://github.com/GitoxideLabs/gitoxide/commit/27aec474c113cc885d44631b329454dc1ad0fed2))
    - Merge pull request #2702 from ameyypawar/fix/2694-exn-source-chain ([`e9c973d`](https://github.com/GitoxideLabs/gitoxide/commit/e9c973d9476bef293bec89cd683cb60b02a85e52))
    - Review ([`dc1fdc3`](https://github.com/GitoxideLabs/gitoxide/commit/dc1fdc3de8a9fb1266c2b5b01a1456bd177e7646))
    - Merge pull request #2618 from GitoxideLabs/report ([`f7d4f33`](https://github.com/GitoxideLabs/gitoxide/commit/f7d4f33b58503996ae90497b69ce4c3a757982ac))
</details>

## 0.2.4 (2026-05-26)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 28 calendar days.
 - 28 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-error v0.2.4, gix-date v0.15.4, gix-actor v0.41.1, gix-trace v0.1.20, gix-validate v0.11.2, gix-path v0.12.1, gix-utils v0.3.3, gix-features v0.48.1, gix-hash v0.25.1, gix-hashtable v0.15.1, gix-object v0.61.0, gix-glob v0.26.1, gix-quote v0.7.2, gix-attributes v0.33.1, gix-command v0.9.1, gix-packetline v0.21.4, gix-filter v0.31.0, gix-fs v0.21.2, gix-chunk v0.7.2, gix-commitgraph v0.37.1, gix-revwalk v0.32.0, gix-traverse v0.58.0, gix-worktree-stream v0.33.0, gix-archive v0.33.0, gix-bitmap v0.3.2, gix-tempfile v23.0.1, gix-lock v23.0.1, gix-index v0.52.0, gix-config-value v0.18.1, gix-pathspec v0.18.1, gix-ignore v0.21.1, gix-worktree v0.53.0, gix-imara-diff v0.2.2, gix-diff v0.64.0, gix-blame v0.14.0, gix-ref v0.64.0, gix-sec v0.14.1, gix-config v0.57.0, gix-prompt v0.15.1, gix-url v0.36.1, gix-credentials v0.38.1, gix-discover v0.52.0, gix-dir v0.26.0, gix-mailmap v0.33.1, gix-revision v0.46.0, gix-merge v0.17.0, gix-negotiate v0.32.0, gix-pack v0.71.0, gix-odb v0.81.0, gix-refspec v0.42.0, gix-shallow v0.12.1, gix-transport v0.57.1, gix-protocol v0.62.0, gix-status v0.31.0, gix-submodule v0.31.0, gix-worktree-state v0.31.0, gix v0.84.0, gix-fsck v0.22.0, gitoxide-core v0.58.0, gitoxide v0.54.0, safety bump 27 crates ([`10c58bb`](https://github.com/GitoxideLabs/gitoxide/commit/10c58bb56597d9335611da121aac21f9b09b6e5b))
    - Merge pull request #2568 from GitoxideLabs/dependabot/cargo/cargo-56d6b174d8 ([`ab2fee1`](https://github.com/GitoxideLabs/gitoxide/commit/ab2fee14651202fcb7b3d8178932090c73492014))
    - Update crates to Rust 2024 edition ([`2cb17b2`](https://github.com/GitoxideLabs/gitoxide/commit/2cb17b2e7f6009693a55af907614f705a29d8c29))
    - Raise MSRV for hash dependency updates ([`3675a8d`](https://github.com/GitoxideLabs/gitoxide/commit/3675a8d61b17845a783bc27912a3f52ac273a4af))
    - Merge pull request #2546 from GitoxideLabs/fix-2545 ([`adb8328`](https://github.com/GitoxideLabs/gitoxide/commit/adb8328952478c443ead5f5a8c6851928b377b37))
</details>

## 0.2.3 (2026-04-28)

### Bug Fixes

 - <csr-id-76a03ebec19ec0a0d45d5ecf67ad49203df26adf/> improve error message around "Signature name or email must not contain..."
   This might make it easier to understand where the error is coming from.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 2 calendar days.
 - 4 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#2491](https://github.com/GitoxideLabs/gitoxide/issues/2491)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2491](https://github.com/GitoxideLabs/gitoxide/issues/2491)**
    - Improve error message around "Signature name or email must not contain..." ([`76a03eb`](https://github.com/GitoxideLabs/gitoxide/commit/76a03ebec19ec0a0d45d5ecf67ad49203df26adf))
 * **Uncategorized**
    - Release gix-error v0.2.3, gix-date v0.15.3, gix-actor v0.41.0, gix-path v0.12.0, gix-features v0.48.0, gix-hash v0.25.0, gix-hashtable v0.15.0, gix-object v0.60.0, gix-glob v0.26.0, gix-attributes v0.33.0, gix-command v0.9.0, gix-filter v0.30.0, gix-fs v0.21.0, gix-commitgraph v0.37.0, gix-revwalk v0.31.0, gix-traverse v0.57.0, gix-worktree-stream v0.32.0, gix-archive v0.32.0, gix-tempfile v23.0.0, gix-lock v23.0.0, gix-index v0.51.0, gix-config-value v0.18.0, gix-pathspec v0.18.0, gix-ignore v0.21.0, gix-worktree v0.52.0, gix-imara-diff v0.2.1, gix-diff v0.63.0, gix-blame v0.13.0, gix-ref v0.63.0, gix-sec v0.14.0, gix-config v0.56.0, gix-prompt v0.15.0, gix-url v0.36.0, gix-credentials v0.38.0, gix-discover v0.51.0, gix-dir v0.25.0, gix-mailmap v0.33.0, gix-revision v0.45.0, gix-merge v0.16.0, gix-negotiate v0.31.0, gix-pack v0.70.0, gix-odb v0.80.0, gix-refspec v0.41.0, gix-shallow v0.12.0, gix-transport v0.57.0, gix-protocol v0.61.0, gix-status v0.30.0, gix-submodule v0.30.0, gix-worktree-state v0.30.0, gix v0.83.0, gix-fsck v0.21.0, gitoxide-core v0.57.0, gitoxide v0.53.0, safety bump 48 crates ([`53f880c`](https://github.com/GitoxideLabs/gitoxide/commit/53f880c7604232c367870088176e42efd8a5b783))
    - Merge pull request #2540 from GitoxideLabs/reporting ([`4d5ba23`](https://github.com/GitoxideLabs/gitoxide/commit/4d5ba231685e8ff36195603c57193aa1cd21fa8e))
    - Merge pull request #2529 from GitoxideLabs/reflog-newline-handling ([`2c3a08e`](https://github.com/GitoxideLabs/gitoxide/commit/2c3a08e7d255df7d939af3d59c42aa0d6a21b76a))
</details>

## 0.2.2 (2026-04-24)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 32 calendar days.
 - 33 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Update changelogs prior to release ([`f9fbcba`](https://github.com/GitoxideLabs/gitoxide/commit/f9fbcba28278f3fb2ad7969c2d00ac6765165724))
    - Merge pull request #2518 from GitoxideLabs/improvements ([`444a92b`](https://github.com/GitoxideLabs/gitoxide/commit/444a92b0fa1df406cf2f36f8dbe82c2859e04e0b))
    - Make `package.include` patterns more specific so they don't match ignored files ([`c2c917f`](https://github.com/GitoxideLabs/gitoxide/commit/c2c917fce56c40a9af0d06bd603b7d1d2e51474f))
    - Merge pull request #2483 from GitoxideLabs/improvements ([`5f5a836`](https://github.com/GitoxideLabs/gitoxide/commit/5f5a836f666bf346050af21a75f22ecd649cc698))
    - Make `just nextest` work reliably ([`789b57f`](https://github.com/GitoxideLabs/gitoxide/commit/789b57f95eaedf9ff58bcd587d99940f22038f25))
    - Merge pull request #2480 from GitoxideLabs/report ([`98bae84`](https://github.com/GitoxideLabs/gitoxide/commit/98bae84fe534879899489c6f2c5e8cfcc863116d))
</details>

## 0.2.1 (2026-03-22)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 28 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-error v0.2.1, gix-date v0.15.1, gix-path v0.11.2, gix-features v0.46.2, gix-hash v0.23.0, gix-hashtable v0.13.0, gix-object v0.58.0, gix-packetline v0.21.2, gix-filter v0.28.0, gix-fs v0.19.2, gix-commitgraph v0.35.0, gix-revwalk v0.29.0, gix-traverse v0.55.0, gix-worktree-stream v0.30.0, gix-archive v0.30.0, gix-tempfile v21.0.2, gix-lock v21.0.2, gix-index v0.49.0, gix-pathspec v0.16.1, gix-ignore v0.19.1, gix-worktree v0.50.0, gix-diff v0.61.0, gix-blame v0.11.0, gix-ref v0.61.0, gix-sec v0.13.2, gix-config v0.54.0, gix-prompt v0.14.1, gix-credentials v0.37.1, gix-discover v0.49.0, gix-dir v0.23.0, gix-revision v0.43.0, gix-merge v0.14.0, gix-negotiate v0.29.0, gix-pack v0.68.0, gix-odb v0.78.0, gix-refspec v0.39.0, gix-shallow v0.10.0, gix-transport v0.55.1, gix-protocol v0.59.0, gix-status v0.28.0, gix-submodule v0.28.0, gix-worktree-state v0.28.0, gix v0.81.0, gix-fsck v0.19.0, gitoxide-core v0.55.0, gitoxide v0.52.0, safety bump 31 crates ([`c389a2c`](https://github.com/GitoxideLabs/gitoxide/commit/c389a2ccb32b36c1178a1352a2bb3229aef3b016))
    - Merge pull request #2454 from GitoxideLabs/dependabot/cargo/cargo-da044b9bb0 ([`6183fd0`](https://github.com/GitoxideLabs/gitoxide/commit/6183fd092d7acd43763fe15be400ce81e7172775))
    - Bump the cargo group with 68 updates ([`6bdb331`](https://github.com/GitoxideLabs/gitoxide/commit/6bdb33145e8aa81ba0dae5caafc675c591569715))
    - Merge pull request #2442 from GitoxideLabs/report ([`f7277f3`](https://github.com/GitoxideLabs/gitoxide/commit/f7277f3c9e3e5130edb714ff5bd3db06b7f589b3))
</details>

## 0.2.0 (2026-02-22)

### Documentation

 - <csr-id-fdf321b9b9c7ca1e762ed3b7ddbe149e55e2e4bb/> add `From<Message>` for `ValidationError` guide.
   This allows to more conveniently create validation errors.

### New Features (BREAKING)

 - <csr-id-502eaa0f750130bbd01112c8486be1f5e576a753/> `gix-error` instead of `thiserror` in `gix-quote`
   Replace the thiserror-derived `ansi_c::undo::Error` enum with
   `gix_error::Exn<gix_error::ValidationError>`, converting the `Error::new()`
   factory and variant constructors to `message!()` calls.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 10 calendar days.
 - 12 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-error v0.2.0, gix-date v0.15.0, gix-actor v0.40.0, gix-object v0.57.0, gix-quote v0.7.0, gix-attributes v0.31.0, gix-command v0.8.0, gix-filter v0.27.0, gix-chunk v0.7.0, gix-commitgraph v0.34.0, gix-revwalk v0.28.0, gix-traverse v0.54.0, gix-worktree-stream v0.29.0, gix-archive v0.29.0, gix-bitmap v0.3.0, gix-index v0.48.0, gix-pathspec v0.16.0, gix-worktree v0.49.0, gix-diff v0.60.0, gix-blame v0.10.0, gix-ref v0.60.0, gix-config v0.53.0, gix-prompt v0.14.0, gix-url v0.35.2, gix-credentials v0.37.0, gix-discover v0.48.0, gix-dir v0.22.0, gix-mailmap v0.32.0, gix-revision v0.42.0, gix-merge v0.13.0, gix-negotiate v0.28.0, gix-pack v0.67.0, gix-odb v0.77.0, gix-refspec v0.38.0, gix-shallow v0.9.0, gix-transport v0.55.0, gix-protocol v0.58.0, gix-status v0.27.0, gix-submodule v0.27.0, gix-worktree-state v0.27.0, gix v0.80.0, gix-fsck v0.18.0, gitoxide-core v0.54.0, gitoxide v0.51.0, safety bump 42 crates ([`ecf90fc`](https://github.com/GitoxideLabs/gitoxide/commit/ecf90fccb9d43bff320c17f46fdc3f5832533a52))
    - Merge pull request #2423 from GitoxideLabs/gix-error ([`000d58a`](https://github.com/GitoxideLabs/gitoxide/commit/000d58a9e3ec680b89186793bd8e09b9704835f5))
    - `gix-error` instead of `thiserror` in `gix-quote` ([`502eaa0`](https://github.com/GitoxideLabs/gitoxide/commit/502eaa0f750130bbd01112c8486be1f5e576a753))
    - Add `From<Message>` for `ValidationError` guide. ([`fdf321b`](https://github.com/GitoxideLabs/gitoxide/commit/fdf321b9b9c7ca1e762ed3b7ddbe149e55e2e4bb))
    - Merge branch 'release' ([`9327b73`](https://github.com/GitoxideLabs/gitoxide/commit/9327b73785227f1322a327cb48fbb0800e1286ae))
</details>

## 0.1.0 (2026-02-10)

### Documentation

 - <csr-id-9007e1b6b8b4b444c1159a2dc9a01242da6ee818/> improve documentation to be more vibe-friendly

### Bug Fixes (BREAKING)

 - <csr-id-b2c516a1689b62e61c9a517f726e5c782cd506b9/> turn `ParseError` into `ValidationError`
   The latter is more general and makes sense both for parsing,
   and for validation.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release over the course of 19 calendar days.
 - 19 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-error v0.1.0, gix-date v0.14.0, gix-actor v0.39.0, gix-trace v0.1.18, gix-path v0.11.1, gix-features v0.46.1, gix-hash v0.22.1, gix-object v0.56.0, gix-quote v0.6.2, gix-attributes v0.30.1, gix-command v0.7.1, gix-packetline v0.21.1, gix-filter v0.26.0, gix-fs v0.19.1, gix-chunk v0.6.0, gix-commitgraph v0.33.0, gix-revwalk v0.27.0, gix-traverse v0.53.0, gix-worktree-stream v0.28.0, gix-archive v0.28.0, gix-bitmap v0.2.16, gix-tempfile v21.0.1, gix-lock v21.0.1, gix-index v0.47.0, gix-config-value v0.17.1, gix-pathspec v0.15.1, gix-worktree v0.48.0, gix-diff v0.59.0, gix-blame v0.9.0, gix-ref v0.59.0, gix-sec v0.13.1, gix-config v0.52.0, gix-prompt v0.13.1, gix-url v0.35.1, gix-credentials v0.36.0, gix-discover v0.47.0, gix-dir v0.21.0, gix-mailmap v0.31.0, gix-revision v0.41.0, gix-merge v0.12.0, gix-negotiate v0.27.0, gix-pack v0.66.0, gix-odb v0.76.0, gix-refspec v0.37.0, gix-shallow v0.8.1, gix-transport v0.54.0, gix-protocol v0.57.0, gix-status v0.26.0, gix-submodule v0.26.0, gix-worktree-state v0.26.0, gix v0.79.0, safety bump 35 crates ([`d66ac10`](https://github.com/GitoxideLabs/gitoxide/commit/d66ac1057a5b7bfb608d4e6be585c69fb692bfee))
    - Merge pull request #2400 from GitoxideLabs/gix-error ([`e4f016b`](https://github.com/GitoxideLabs/gitoxide/commit/e4f016bd386deae6466bf703ba0b7959e6460ac8))
    - Refactor2 ([`f860c0b`](https://github.com/GitoxideLabs/gitoxide/commit/f860c0b5f5fe316464baaf6e6487e8cb394b78e8))
    - Address Copilot review ([`0b0e9f8`](https://github.com/GitoxideLabs/gitoxide/commit/0b0e9f8df95e60626cfec2f8665af072b4ddc77c))
    - Improve documentation to be more vibe-friendly ([`9007e1b`](https://github.com/GitoxideLabs/gitoxide/commit/9007e1b6b8b4b444c1159a2dc9a01242da6ee818))
    - Merge pull request #2407 from GitoxideLabs/dependabot/cargo/cargo-fb4135702f ([`8bceefb`](https://github.com/GitoxideLabs/gitoxide/commit/8bceefbfc5f897517bfdd24744695a82cfa0d5be))
    - Bump the cargo group with 59 updates ([`7ce3c55`](https://github.com/GitoxideLabs/gitoxide/commit/7ce3c5587aec1ca813039c047783b9cb2a106826))
    - Merge pull request #2396 from GitoxideLabs/gix-error ([`e8612b5`](https://github.com/GitoxideLabs/gitoxide/commit/e8612b5bd16eb19a04ddf7e37d94bef013127f88))
    - Turn `ParseError` into `ValidationError` ([`b2c516a`](https://github.com/GitoxideLabs/gitoxide/commit/b2c516a1689b62e61c9a517f726e5c782cd506b9))
    - Merge pull request #2393 from GitoxideLabs/report ([`f7d0975`](https://github.com/GitoxideLabs/gitoxide/commit/f7d09758d245aaa89409e39bb6ba1ed6b7118ea5))
</details>

## 0.0.0 (2026-01-22)

### New Features

 - <csr-id-461c87667c75a9db0a74c43ef68d71b88a7dd754/> Add an `auto-chain-error` feature to let `gix-error::Error` produce error chains suitable for `anyhow`.
 - <csr-id-28f4211afadd91c5b5d2d2a0698f37e660cc0c66/> make it possible to produce errors that work well with `anyhow` source-chain display.
 - <csr-id-053c3ee2217480eead3aa7c71fa4b65455444921/> anyhow support for `gix-error::Exn`
   This is mainly useful for `gitoxide-core`, which may call plumbing.
 - <csr-id-3301eb8b2906861952726061629d74babfd24f73/> Add `Exn::downcast_any_ref()`

### New Features (BREAKING)

 - <csr-id-5ab19a7a3344c58ad1185a23a789848ed5e02241/> Use `gix-error` in `gix-date`
   This will make for easier introspection for users of these errors.

### Refactor (BREAKING)

 - <csr-id-829393ac596bf2684bd8a837ae931773b24ee033/> ErrorExt::raise_iter to raise_all + remove Frame::downcast
   Be more compatible to `exn`.
 - <csr-id-f8517bedcbb9b3328f435aa37f4c63bd30b19fc0/> catch up Exn designs with the upstream
   refactor!: rename `Exn::from_iter` to `raise_all`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 26 commits contributed to the release over the course of 12 calendar days.
 - 7 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#2384](https://github.com/GitoxideLabs/gitoxide/issues/2384), [#2385](https://github.com/GitoxideLabs/gitoxide/issues/2385)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2384](https://github.com/GitoxideLabs/gitoxide/issues/2384)**
    - Catch up Exn designs with the upstream ([`f8517be`](https://github.com/GitoxideLabs/gitoxide/commit/f8517bedcbb9b3328f435aa37f4c63bd30b19fc0))
 * **[#2385](https://github.com/GitoxideLabs/gitoxide/issues/2385)**
    - ErrorExt::raise_iter to raise_all + remove Frame::downcast ([`829393a`](https://github.com/GitoxideLabs/gitoxide/commit/829393ac596bf2684bd8a837ae931773b24ee033))
 * **Uncategorized**
    - Fixes to make a release work. ([`fa302a1`](https://github.com/GitoxideLabs/gitoxide/commit/fa302a115918289ca2c4b33f5aa576f478e46092))
    - Merge pull request #2383 from GitoxideLabs/gix-error ([`9d39656`](https://github.com/GitoxideLabs/gitoxide/commit/9d39656710c297f9a22e4a7e6facc3a1f35f89e0))
    - Address Copilot review ([`16327ef`](https://github.com/GitoxideLabs/gitoxide/commit/16327efe24e2321e2a4efe5321e9f0483484b10a))
    - Add an `auto-chain-error` feature to let `gix-error::Error` produce error chains suitable for `anyhow`. ([`461c876`](https://github.com/GitoxideLabs/gitoxide/commit/461c87667c75a9db0a74c43ef68d71b88a7dd754))
    - Make it possible to produce errors that work well with `anyhow` source-chain display. ([`28f4211`](https://github.com/GitoxideLabs/gitoxide/commit/28f4211afadd91c5b5d2d2a0698f37e660cc0c66))
    - Anyhow support for `gix-error::Exn` ([`053c3ee`](https://github.com/GitoxideLabs/gitoxide/commit/053c3ee2217480eead3aa7c71fa4b65455444921))
    - Merge pull request #2378 from GitoxideLabs/gix-error ([`6cff657`](https://github.com/GitoxideLabs/gitoxide/commit/6cff65786b5213194fffd2c77b7c2dc44dcb4b52))
    - Address Copilot review ([`e112cac`](https://github.com/GitoxideLabs/gitoxide/commit/e112cacc42a192d5159b299e49739f3af2589e3e))
    - Change `ErrorExt::erased()` to `ErrorExt::raise_erased()`. ([`373fced`](https://github.com/GitoxideLabs/gitoxide/commit/373fceddcc1a0ef79f306b519a2ca3682b3110ef))
    - Make `Exn` work properly after the type was erased. ([`499402c`](https://github.com/GitoxideLabs/gitoxide/commit/499402c941e85e6cff5c3ffef8a09afac842c7ac))
    - Add `Exn::downcast_any_ref()` ([`3301eb8`](https://github.com/GitoxideLabs/gitoxide/commit/3301eb8b2906861952726061629d74babfd24f73))
    - Merge pull request #2374 from GitoxideLabs/gix-error ([`25233ce`](https://github.com/GitoxideLabs/gitoxide/commit/25233ced7f17e14842aa400cf007a0feb6127d89))
    - Turn `Exn::into_box()` to `Exn::into_inner()`. ([`939b8fc`](https://github.com/GitoxideLabs/gitoxide/commit/939b8fcbb2115eba77aca1be8527ad0d7f644c56))
    - Merge pull request #2373 from GitoxideLabs/gix-error ([`4c6a7a7`](https://github.com/GitoxideLabs/gitoxide/commit/4c6a7a76c214c94910f141542d677dc2a7500ddd))
    - Adapt to changes in `gix-chunk` ([`e6e90ff`](https://github.com/GitoxideLabs/gitoxide/commit/e6e90ff82b1f839a6d78170685f2a69566766675))
    - Add conversion from Message to `ParseError` for less noisy invocations. ([`08f9ed4`](https://github.com/GitoxideLabs/gitoxide/commit/08f9ed48c896ea92d8d8da9b15bc44f7709b013e))
    - More docs to better explain `gix-error` ([`f46ca99`](https://github.com/GitoxideLabs/gitoxide/commit/f46ca9925930e4b2a660d2896ccbfb8edd3aa4e9))
    - Merge pull request #2352 from GitoxideLabs/gix-error ([`ae23762`](https://github.com/GitoxideLabs/gitoxide/commit/ae23762932ea0d78e91463185a304d778746a167))
    - Make it possible to traverse frames using an iterator. ([`3bac149`](https://github.com/GitoxideLabs/gitoxide/commit/3bac149385a6f64ab0ee1989ad562132574dc021))
    - Actually introduce `gix-error` into `gix-revision`. ([`4819ea8`](https://github.com/GitoxideLabs/gitoxide/commit/4819ea8d81645b8b79dc2a3fcba7b27d773a9fce))
    - Adadpt `exn` to most pressing needs of `gitoxide` ([`abedade`](https://github.com/GitoxideLabs/gitoxide/commit/abedadec5463b57e78aa53e62d8c511b989ae9ca))
    - Vendor `exn` from https://github.com/fast/exn@bb4d8ea4e4df335c46d4fa3f4f260121f9f84305 ([`0eaab70`](https://github.com/GitoxideLabs/gitoxide/commit/0eaab70ee6e897635d7fb41402ec87387b8ecd4b))
    - Use `gix-error` in `gix-date` ([`5ab19a7`](https://github.com/GitoxideLabs/gitoxide/commit/5ab19a7a3344c58ad1185a23a789848ed5e02241))
    - Create a basic `gix-error` crate to forward `exn` ([`35cf1ff`](https://github.com/GitoxideLabs/gitoxide/commit/35cf1ff837ea30a1366b20bde0d59baf9ab699be))
</details>

