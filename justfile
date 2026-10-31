#!/usr/bin/env -S just --justfile
# ^ A shebang isn't required, but allows a justfile to be executed
#   like a script, with `./justfile test`, for example.

j := quote(just_executable())

# List available recipes
[private]
default:
    @{{ j }} --list --unsorted

alias t := test
alias c := check
alias nt := nextest

# Show affected packages and why; use --base for a stacked PR or --mode exhaustive
[group('CI')]
[positional-arguments]
ci-plan *args:
    cargo run --quiet --locked -p internal-tools --no-default-features --bin ci -- plan "$@"

# Reproduce CI on this host; --dry-run prints commands, --stage selects checks
[group('CI')]
[positional-arguments]
ci *args:
    cargo run --quiet --locked -p internal-tools --no-default-features --bin ci -- run "$@"

# Run the full check and test suite, including docs and journey tests
[group('Development')]
[positional-arguments]
test *args:
    cargo run --locked -p internal-tools --no-default-features --bin ci -- run --mode exhaustive "$@"

# Run `cargo clippy` on all crates
[group('Development')]
clippy *clippy-args:
    cargo clippy --workspace --all-targets -- {{ clippy-args }}
    cargo clippy --workspace --no-default-features --features small -- {{ clippy-args }}
    cargo clippy --workspace --no-default-features --features max-pure -- {{ clippy-args }}
    cargo clippy --workspace --no-default-features --features lean-async --tests -- {{ clippy-args }}

# Apply Clippy fixes to all crates, then format
[group('Development')]
clippy-fix:
    cargo clippy --fix --workspace --all-targets
    cargo clippy --fix --allow-dirty --workspace --no-default-features --features small
    cargo clippy --fix --allow-dirty --workspace --no-default-features --features max-pure
    cargo clippy --fix --allow-dirty --workspace --no-default-features --features lean-async --tests
    cargo fmt --all

# Build all code in suitable configurations
[group('Development')]
check:
    etc/scripts/cargo-check-all.sh

# Check stable APIs against a revision, or the latest supported release (Rust 1.99)
[group('Development')]
check-api baseline='':
    cargo test -p internal-tools --test check_api -- --ignored
    etc/scripts/check-gix-api.sh
    bash etc/scripts/check-api-compare.sh {{ quote(baseline) }}

# Check cargo-auditable compatibility (install with `cargo install cargo-auditable --locked`)
[group('Development')]
check-auditable:
    # Keep this independent of compatibility aliases so they can be removed after upstream fixes.
    cargo auditable build --locked --no-default-features --features small

# Run `cargo doc` on all crates
[group('Development')]
doc $RUSTDOCFLAGS='-D warnings':
    # Default max already enables the lean/small dependency features.
    cargo doc --locked --workspace --no-deps

# Run all unit tests
[group('Tests')]
unit-tests:
    cargo run --locked -p internal-tools --no-default-features --bin ci -- run --mode exhaustive --stage tests

# Run all doctests
[group('Tests')]
doc-tests:
    cargo run --locked -p internal-tools --no-default-features --bin ci -- run --mode exhaustive --stage doctests

# Run optional flaky async tests
[group('Tests')]
unit-tests-flaky:
    cargo test -p gix --features async-network-client-async-std,sha256

# Extract cargo metadata, excluding dependencies, and query it
[private]
query-meta jq-query:
    meta="$(cargo metadata --format-version 1 --no-deps)" && \
        printf '%s\n' "$meta" | jq --exit-status --raw-output -- {{ quote(jq-query) }}

# Get the path to the directory where debug binaries are created during builds
[private]
dbg: (query-meta '.target_directory + "/debug"')

# Run journey tests (`max`)
[group('Tests')]
journey-tests:
    # Keep tree-shaped error reports in journey snapshots.
    cargo build --no-default-features --features max,http-client-curl-rustls
    cargo build -p gix-testtools --bin jtt --features sha1
    dbg="$({{ j }} dbg)" && tests/journey.sh "$dbg/ein" "$dbg/gix" "$dbg/jtt" max

# Run journey tests (`max-pure`)
[group('Tests')]
journey-tests-pure:
    cargo build --no-default-features --features max-pure
    cargo build -p gix-testtools --bin jtt --features sha1
    dbg="$({{ j }} dbg)" && tests/journey.sh "$dbg/ein" "$dbg/gix" "$dbg/jtt" max-pure

# Run journey tests (`small`)
[group('Tests')]
journey-tests-small:
    cargo build --no-default-features --features small
    cargo build -p gix-testtools --features sha1
    dbg="$({{ j }} dbg)" && tests/journey.sh "$dbg/ein" "$dbg/gix" "$dbg/jtt" small

# Run journey tests (`lean-async`)
[group('Tests')]
journey-tests-async:
    cargo build --no-default-features --features lean-async
    cargo build -p gix-testtools --features sha1
    dbg="$({{ j }} dbg)" && tests/journey.sh "$dbg/ein" "$dbg/gix" "$dbg/jtt" async

# Build a customized `cross` container image for testing
[group('Cross compilation')]
cross-image target:
    docker build --build-arg "TARGET={{ target }}" \
        -t "cross-rs-gitoxide:{{ target }}" \
        -f etc/docker/Dockerfile.test-cross etc/docker/test-cross-context

# Test another platform with `cross`
[group('Cross compilation')]
cross-test target options test-options: (cross-image target)
    CROSS_CONFIG=etc/docker/test-cross.toml NO_PRELOAD_CXX=1 \
        cross test --workspace --no-fail-fast --target {{ target }} \
        {{ options }} -- --skip realpath::fuzzed_timeout {{ test-options }}

# Test s390x with `cross`
[group('Cross compilation')]
cross-test-s390x: (cross-test 's390x-unknown-linux-gnu' '' '')

# Test Android with `cross` (max-pure)
[group('Cross compilation')]
cross-test-android: (cross-test 'armv7-linux-androideabi' '--no-default-features --features max-pure' '')

# Check crate package sizes with `cargo diet`
[group('Dependencies and SBOMs')]
check-size:
    etc/scripts/check-package-size.sh

# Print the minimum Rust version for `gix` (X.Y.Z)
[group('Maintenance')]
msrv: (query-meta '''
    .packages[]
    | select(.name == "gix")
    | .rust_version
    | sub("(?<xy>^[0-9]+[.][0-9]+$)"; "\(.xy).0")
''')

# Regenerate the MSRV badge SVG
[group('Maintenance')]
msrv-badge:
    msrv="$({{ j }} msrv)" && \
        sed "s/{MSRV}/$msrv/g" etc/msrv-badge.template.svg >etc/msrv-badge.svg

# Build `gix` and its locked dependencies with this Rust version
[group('Maintenance')]
check-rust-version rust-version:
    rustc +{{ rust-version }} --version
    cargo +{{ rust-version }} build --locked -p gix
    cargo +{{ rust-version }} build --locked -p gix \
        --no-default-features --features async-network-client,max-performance,sha1

# Enter a nix-shell able to build on macOS
[group('Maintenance')]
nix-shell-macos:
    nix-shell -p pkg-config openssl libiconv darwin.apple_sdk.frameworks.Security darwin.apple_sdk.frameworks.SystemConfiguration

# Audit dependency advisories, bans, licenses, and sources
[group('Dependencies and SBOMs')]
audit:
    cargo deny --workspace --all-features --locked check advisories bans licenses sources

# Install the pinned Rust tools for CycloneDX and SPDX SBOMs
[group('Dependencies and SBOMs')]
sbom-install:
    cargo install --locked --version 0.5.9 cargo-cyclonedx
    cargo install --locked --no-default-features --features cli --version 0.2.0 sbom-tools

# Generate CycloneDX and SPDX SBOMs (--help for crate and feature options)
[group('Dependencies and SBOMs')]
[positional-arguments]
sbom *args:
    cargo run --locked -p gix-testtools --bin jtt --features sha1,sbom -- sbom "$@"

# Test SBOM generation with the tools installed by sbom-install
[group('Dependencies and SBOMs')]
sbom-test:
    cargo test --locked -p gix-testtools --features sha1,sbom --test sbom -- --include-ignored

# Run unit tests with `cargo nextest` (no doctests)
[group('Development')]
nextest *FLAGS='--workspace':
    cargo nextest run {{ FLAGS }}

# Run nextest tests, including ignored tests, without status reports
[group('Tests')]
summarize EXPRESSION='all()':
    cargo nextest run --workspace --run-ignored all --no-fail-fast \
        --status-level none --final-status-level none -E {{ quote(EXPRESSION) }}

# Format with nightly Rust, check stable formatting, and format the justfile
[group('Development')]
fmt:
    cargo +nightly fmt --all -- --config-path rustfmt-nightly.toml
    cargo +stable fmt --all -- --check
    {{ j }} --fmt --unstable

# Look for yanked crates (cancel after the initial warnings)
[group('Dependencies and SBOMs')]
find-yanked:
    cargo install --debug --locked --no-default-features --features max-pure --path .

# Check shell scripts' executable bits and shebangs
[group('Maintenance')]
check-mode:
    cargo run --locked -p internal-tools --no-default-features --bin ci -- exec mode

# Clean the `target` directory
[group('Maintenance')]
clear-target:
    cargo clean

# Publish crates, allowing empty release messages and fully generated changelogs
[group('Releases')]
[positional-arguments]
release *args:
    cargo smart-release --publish-uses-docs-rs-metadata --allow-empty-release-message --allow-fully-generated-changelogs --auto-publish-of-stable-crates "$@"

# Get the unique `v*` tag at `HEAD`, or fail with an error
[group('Releases')]
unique-v-tag:
    etc/scripts/unique-v-tag.sh

# Trigger the `release.yml` workflow on the current `v*` tag
[group('Releases')]
run-release-workflow repo='':
    optional_repo_arg={{ quote(repo) }} && \
        export GH_REPO="${optional_repo_arg:-"${GH_REPO:-GitoxideLabs/gitoxide}"}" && \
        tag_name="$({{ j }} unique-v-tag)" && \
        printf 'Running release.yml in %s repo for %s tag.\n' "$GH_REPO" "$tag_name" && \
        gh workflow run release.yml --ref "refs/tags/$tag_name"

# Run `cargo smart-release` and then trigger `release.yml` for the `v*` tag
[group('Releases')]
roll-release *csr-args:
    cargo smart-release {{ csr-args }}
    {{ j }} run-release-workflow

# Run checks, docs, and unit tests for CI
[group('CI')]
ci-test:
    cargo run --locked -p internal-tools --no-default-features --bin ci -- run --mode exhaustive --stage check,docs,doctests,tests,housekeeping

# Run all journey configurations; each build stays adjacent to its tests
[group('CI')]
ci-journey-tests: journey-tests-pure journey-tests-small journey-tests-async journey-tests
