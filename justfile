#!/usr/bin/env -S just --justfile
# ^ A shebang isn't required, but allows a justfile to be executed
#   like a script, with `./justfile test`, for example.

j := quote(just_executable())

# List available recipes
default:
    {{ j }} --list

alias t := test
alias c := check
alias nt := nextest

# Run all tests, clippy, including journey tests, try building docs
test: clippy check doc unit-tests doc-tests journey-tests-pure journey-tests-small journey-tests-async journey-tests check-mode

# Run all tests, without clippy, and try building docs
ci-test: check doc unit-tests check-mode

# Run dynamic ODB state-change scenarios with contending handles
test-odb-threaded-scenarios:
    env GIX_ODB_TEST_THREADS=8 cargo test -p gix-odb --features parallel --test odb store::dynamic_scenarios:: -- --test-threads=1

# Run all journey tests - should be run in a fresh clone or after `cargo clean`
ci-journey-tests: journey-tests-pure journey-tests-small journey-tests-async journey-tests

# Clean the `target` directory
clear-target:
    cargo clean

# Run `cargo clippy` on all crates
clippy *clippy-args:
    cargo clippy --workspace --all-targets -- {{ clippy-args }}
    cargo clippy --workspace --no-default-features --features small -- {{ clippy-args }}
    cargo clippy --workspace --no-default-features --features max-pure -- {{ clippy-args }}
    cargo clippy --workspace --no-default-features --features lean-async --tests -- {{ clippy-args }}

# Run `cargo clippy` on all crates, fixing what can be fixed, and format all code
clippy-fix:
    cargo clippy --fix --workspace --all-targets
    cargo clippy --fix --allow-dirty --workspace --no-default-features --features small
    cargo clippy --fix --allow-dirty --workspace --no-default-features --features max-pure
    cargo clippy --fix --allow-dirty --workspace --no-default-features --features lean-async --tests
    cargo fmt --all

# Build all code in suitable configurations
check:
    etc/scripts/cargo-check-all.sh

# Run `cargo doc` on all crates
doc $RUSTDOCFLAGS='-D warnings':
    cargo doc --workspace --no-deps
    cargo doc --features=max,lean,small --workspace --no-deps

# Run all unit tests
unit-tests:
    cargo nextest run --no-fail-fast
    cargo nextest run -p gix-attributes --features serde --no-fail-fast
    # Test repository snapshots with the default pure-gix backend and the Git CLI backend.
    cargo nextest run -p gix-testtools --no-fail-fast
    cargo nextest run -p gix-testtools --no-default-features --features worktree-exclusions,sha1,sha256 --no-fail-fast
    cargo nextest run -p gix-testtools --features xz --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-archive --no-default-features --features sha1 --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-archive --no-default-features --features sha1,tar --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-archive --no-default-features --features sha1,tar_gz --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-archive --no-default-features --features sha1,zip --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-archive --features sha256 --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-archive --no-default-features --features sha256 --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-archive --no-default-features --features sha256,tar --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-archive --no-default-features --features sha256,tar_gz --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-archive --no-default-features --features sha256,zip --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-diff --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-diff --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-status --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-status --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-dir --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-dir --features sha256 --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-worktree-state --features parallel --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-worktree-state --features sha256,parallel --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-worktree --features parallel --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-worktree --features sha256,parallel --no-fail-fast
    cargo nextest run -p gix-error --no-fail-fast --test auto-chain-error --features auto-chain-error
    cargo nextest run -p gix-error --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-filter --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-filter --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-fsck --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-fsck --features sha256 --no-fail-fast
    cargo nextest run -p gix-hash --features sha1 --no-fail-fast
    cargo nextest run -p gix-hash --features sha1,sha256 --no-fail-fast
    cargo nextest run -p gix-hash --features sha256 --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-commitgraph --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-commitgraph --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-object --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-object --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-object --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-object --no-fail-fast
    cargo nextest run -p gix-tempfile --features signals --no-fail-fast
    cargo nextest run -p gix-features --all-features --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-ref --all-features --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-ref --all-features --no-fail-fast
    cargo nextest run -p gix-odb --all-features --no-fail-fast
    cargo nextest run -p gix-odb --features parallel --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-odb --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-odb --no-fail-fast
    # cover the parallel regression test under SHA-256, SHA-1 is covered by --features parallel above
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-odb --features parallel --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-pack --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-pack --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-diff --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-diff --no-fail-fast
    cargo nextest run -p gix-pack --features parallel --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-index --features parallel --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-index --features parallel --no-fail-fast
    cargo nextest run -p gix-packetline --features blocking-io --test blocking-packetline --no-fail-fast
    cargo nextest run -p gix-packetline --features async-io --test async-packetline --no-fail-fast
    cargo nextest run -p gix-transport --features http-client-curl --no-fail-fast
    cargo nextest run -p gix-transport --features http-client-curl,http-client-insecure-credentials --test blocking-transport-http-only --no-fail-fast
    cargo nextest run -p gix-transport --features http-client-reqwest --no-fail-fast
    cargo nextest run -p gix-transport --no-default-features --features blocking-client,http-client-reqwest,http-client-insecure-credentials --test blocking-transport --no-fail-fast
    cargo nextest run -p gix-transport --features async-client --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-traverse --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-traverse --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-merge --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-merge --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-negotiate --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-negotiate --features sha256 --no-fail-fast
    cargo nextest run -p gix-protocol --features blocking-client --no-fail-fast
    cargo nextest run -p gix-protocol --features blocking-client,sha256 --no-fail-fast
    cargo nextest run -p gix-protocol --features async-client --no-fail-fast
    cargo nextest run -p gix-protocol --features async-client,sha256 --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-blame --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-blame --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-refspec --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-refspec --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-revision --features sha256 --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-revision --features sha256 --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha1 cargo nextest run -p gix-worktree-stream --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix-worktree-stream --features sha256 --no-fail-fast
    cargo nextest run -p gix --no-default-features --features basic,comfort,max-performance-safe --no-fail-fast
    cargo nextest run -p gix --no-default-features --features basic,extras,comfort --no-fail-fast
    cargo nextest run -p gix --features async-network-client --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix --features async-network-client --no-fail-fast
    cargo nextest run -p gix --features blocking-network-client --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix --features blocking-network-client --no-fail-fast
    env GIX_TEST_FIXTURE_HASH=sha256 cargo nextest run -p gix --no-fail-fast
    cargo nextest run -p gix --no-default-features --features sha256 --lib --no-fail-fast
    cargo nextest run -p gitoxide-core --lib --no-tests=warn --no-fail-fast

# Run all doctests
doc-tests:
    cargo test --workspace --doc --no-fail-fast
    # `cargo nextest` doesn't run doctests, so cover feature-gated examples explicitly here.
    cargo test -p gix-packetline --doc --features blocking-io --no-fail-fast
    cargo test -p gix --doc --no-default-features --no-fail-fast
    cargo test -p gix --doc --no-default-features --features revision --no-fail-fast

# These tests aren't run by default as they are flaky (even locally)
unit-tests-flaky:
    cargo test -p gix --features async-network-client-async-std

# Extract cargo metadata, excluding dependencies, and query it
[private]
query-meta jq-query:
    meta="$(cargo metadata --format-version 1 --no-deps)" && \
        printf '%s\n' "$meta" | jq --exit-status --raw-output -- {{ quote(jq-query) }}

# Get the path to the directory where debug binaries are created during builds
[private]
dbg: (query-meta '.target_directory + "/debug"')

# Run journey tests (`max`)
journey-tests:
    cargo build --features http-client-curl-rustls
    cargo build -p gix-testtools --bin jtt --features sha1
    dbg="$({{ j }} dbg)" && tests/journey.sh "$dbg/ein" "$dbg/gix" "$dbg/jtt" max

# Run journey tests (`max-pure`)
journey-tests-pure:
    cargo build --no-default-features --features max-pure
    cargo build -p gix-testtools --bin jtt --features sha1
    dbg="$({{ j }} dbg)" && tests/journey.sh "$dbg/ein" "$dbg/gix" "$dbg/jtt" max-pure

# Run journey tests (`small`)
journey-tests-small:
    cargo build --no-default-features --features small
    cargo build -p gix-testtools --features sha1
    dbg="$({{ j }} dbg)" && tests/journey.sh "$dbg/ein" "$dbg/gix" "$dbg/jtt" small

# Run journey tests (`lean-async`)
journey-tests-async:
    cargo build --no-default-features --features lean-async
    cargo build -p gix-testtools --features sha1
    dbg="$({{ j }} dbg)" && tests/journey.sh "$dbg/ein" "$dbg/gix" "$dbg/jtt" async

# Build a customized `cross` container image for testing
cross-image target:
    docker build --build-arg "TARGET={{ target }}" \
        -t "cross-rs-gitoxide:{{ target }}" \
        -f etc/docker/Dockerfile.test-cross etc/docker/test-cross-context

# Test another platform with `cross`
cross-test target options test-options: (cross-image target)
    CROSS_CONFIG=etc/docker/test-cross.toml NO_PRELOAD_CXX=1 \
        cross test --workspace --no-fail-fast --target {{ target }} \
        {{ options }} -- --skip realpath::fuzzed_timeout {{ test-options }}

# Test s390x with `cross`
cross-test-s390x: (cross-test 's390x-unknown-linux-gnu' '' '')

# Test Android with `cross` (max-pure)
cross-test-android: (cross-test 'armv7-linux-androideabi' '--no-default-features --features max-pure' '')

# Run `cargo diet` on all crates to see that they are still in bounds
check-size:
    etc/scripts/check-package-size.sh

# Report the Minimum Supported Rust Version (the `rust-version` of `gix`) in X.Y.Z form
msrv: (query-meta '''
    .packages[]
    | select(.name == "gix")
    | .rust_version
    | sub("(?<xy>^[0-9]+[.][0-9]+$)"; "\(.xy).0")
''')

# Regenerate the MSRV badge SVG
msrv-badge:
    msrv="$({{ j }} msrv)" && \
        sed "s/{MSRV}/$msrv/g" etc/msrv-badge.template.svg >etc/msrv-badge.svg

# Check if `gix` and its dependencies, as currently locked, build with `rust-version`
check-rust-version rust-version:
    rustc +{{ rust-version }} --version
    cargo +{{ rust-version }} build --locked -p gix
    cargo +{{ rust-version }} build --locked -p gix \
        --no-default-features --features async-network-client,max-performance,sha1

# Enter a nix-shell able to build on macOS
nix-shell-macos:
    nix-shell -p pkg-config openssl libiconv darwin.apple_sdk.frameworks.Security darwin.apple_sdk.frameworks.SystemConfiguration

# Run various auditing tools to help us stay legal and safe
audit:
    cargo deny --workspace --all-features check advisories bans licenses sources

# Run tests with `cargo nextest` (all unit-tests, no doc-tests, faster)
nextest *FLAGS='--workspace':
    cargo nextest run {{ FLAGS }}

# Run tests with `cargo nextest`, skipping none except as filtered, omitting status reports
summarize EXPRESSION='all()':
    cargo nextest run --workspace --run-ignored all --no-fail-fast \
        --status-level none --final-status-level none -E {{ quote(EXPRESSION) }}

# Run nightly `rustfmt` for its extra features, but check that it won't upset stable `rustfmt`
fmt:
    cargo +nightly fmt --all -- --config-path rustfmt-nightly.toml
    cargo +stable fmt --all -- --check
    {{ j }} --fmt --unstable

# Cancel this after the first few seconds, as yanked crates will appear in warnings
find-yanked:
    cargo install --debug --locked --no-default-features --features max-pure --path .

# Find shell scripts whose +x/-x bits and magic bytes (e.g. `#!`) disagree
check-mode:
    cargo build -p internal-tools
    cargo run -p internal-tools -- check-mode

# Get the unique `v*` tag at `HEAD`, or fail with an error
unique-v-tag:
    etc/scripts/unique-v-tag.sh

# Trigger the `release.yml` workflow on the current `v*` tag
run-release-workflow repo='':
    optional_repo_arg={{ quote(repo) }} && \
        export GH_REPO="${optional_repo_arg:-"${GH_REPO:-GitoxideLabs/gitoxide}"}" && \
        tag_name="$({{ j }} unique-v-tag)" && \
        printf 'Running release.yml in %s repo for %s tag.\n' "$GH_REPO" "$tag_name" && \
        gh workflow run release.yml --ref "refs/tags/$tag_name"

# Run `cargo smart-release` and then trigger `release.yml` for the `v*` tag
roll-release *csr-args:
    cargo smart-release {{ csr-args }}
    {{ j }} run-release-workflow
