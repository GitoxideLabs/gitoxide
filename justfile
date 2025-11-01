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
test: clippy check doc unit-tests journey-tests-pure journey-tests-small journey-tests-async journey-tests check-mode

# Run all tests, without clippy, and try building docs
ci-test: check doc unit-tests check-mode

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
    cargo check --workspace
    cargo check --no-default-features --features small
    cargo check -p gix-packetline --all-features 2>/dev/null
    cargo check -p gix-transport --all-features 2>/dev/null
    # assure compile error occurs
    ! cargo check --features lean-async 2>/dev/null
    ! cargo check -p gitoxide-core --all-features 2>/dev/null
    ! cargo check -p gix-protocol --all-features 2>/dev/null
    # warning happens if nothing found, no exit code :/
    cargo --color=never tree -p gix --no-default-features -e normal -i imara-diff \
        2>&1 >/dev/null | grep '^warning: nothing to print\>'
    cargo --color=never tree -p gix --no-default-features -e normal -i gix-submodule \
        2>&1 >/dev/null | grep '^warning: nothing to print\>'
    cargo --color=never tree -p gix --no-default-features -e normal -i gix-pathspec \
        2>&1 >/dev/null | grep '^warning: nothing to print\>'
    cargo --color=never tree -p gix --no-default-features -e normal -i gix-filter \
        2>&1 >/dev/null | grep '^warning: nothing to print\>'
    ! cargo tree -p gix --no-default-features -i gix-credentials 2>/dev/null
    cargo check --no-default-features --features lean
    cargo check --no-default-features --features lean-async
    cargo check --no-default-features --features max
    cargo check -p gitoxide-core --features blocking-client
    cargo check -p gitoxide-core --features async-client
    cargo check -p gix-pack --no-default-features
    cargo check -p gix-pack --no-default-features --features generate
    cargo check -p gix-pack --no-default-features --features streaming-input
    cargo check -p gix-hash --all-features
    cargo check -p gix-hash
    cargo check -p gix-object --all-features
    cargo check -p gix-object --features verbose-object-parsing-errors
    cargo check -p gix-attributes --features serde
    cargo check -p gix-glob --features serde
    cargo check -p gix-worktree --features serde
    cargo check -p gix-worktree --no-default-features
    cargo check -p gix-actor --features serde
    cargo check -p gix-date --features serde
    cargo check -p gix-tempfile --features signals
    cargo check -p gix-tempfile --features hp-hashmap
    cargo check -p gix-pack --features serde
    cargo check -p gix-pack --features pack-cache-lru-static
    cargo check -p gix-pack --features pack-cache-lru-dynamic
    cargo check -p gix-pack --features object-cache-dynamic
    cargo check -p gix-packetline --features blocking-io
    cargo check -p gix-packetline --features async-io
    cargo check -p gix-index --features serde
    cargo check -p gix-credentials --features serde
    cargo check -p gix-sec --features serde
    cargo check -p gix-revision --features serde
    cargo check -p gix-revision --no-default-features --features describe
    cargo check -p gix-mailmap --features serde
    cargo check -p gix-url --all-features
    cargo check -p gix-status
    cargo check -p gix-status --all-features
    cargo check -p gix-features --all-features
    cargo check -p gix-features --features parallel
    cargo check -p gix-features --features fs-read-dir
    cargo check -p gix-features --features progress
    cargo check -p gix-features --features io-pipe
    cargo check -p gix-features --features crc32
    cargo check -p gix-features --features zlib
    cargo check -p gix-features --features cache-efficiency-debug
    cargo check -p gix-commitgraph --all-features
    cargo check -p gix-config-value --all-features
    cargo check -p gix-config --all-features
    cargo check -p gix-diff --no-default-features
    cargo check -p gix-transport --features blocking-client
    cargo check -p gix-transport --features async-client
    cargo check -p gix-transport --features async-client,async-std
    cargo check -p gix-transport --features http-client
    cargo check -p gix-transport --features http-client-curl
    cargo check -p gix-transport --features http-client-reqwest
    cargo check -p gix-protocol --features blocking-client
    cargo check -p gix-protocol --features async-client
    cargo check -p gix --no-default-features --features async-network-client
    cargo check -p gix --no-default-features --features async-network-client-async-std
    cargo check -p gix --no-default-features --features blocking-network-client
    cargo check -p gix --no-default-features --features blocking-http-transport-curl
    cargo check -p gix --no-default-features --features blocking-http-transport-reqwest
    cargo check -p gix --no-default-features --features max-performance --tests
    cargo check -p gix --no-default-features --features max-performance-safe --tests
    cargo check -p gix --no-default-features --features progress-tree --tests
    cargo check -p gix --no-default-features --features blob-diff --tests
    cargo check -p gix --no-default-features --features revision --tests
    cargo check -p gix --no-default-features --features revparse-regex --tests
    cargo check -p gix --no-default-features --features mailmap --tests
    cargo check -p gix --no-default-features --features excludes --tests
    cargo check -p gix --no-default-features --features attributes --tests
    cargo check -p gix --no-default-features --features worktree-mutation --tests
    cargo check -p gix --no-default-features --features credentials --tests
    cargo check -p gix --no-default-features --features index --tests
    cargo check -p gix --no-default-features --features interrupt --tests
    cargo check -p gix --no-default-features --features blame --tests
    cargo check -p gix --no-default-features
    cargo check -p gix-odb --features serde
    cargo check --no-default-features --features max-control

# Run `cargo doc` on all crates
doc $RUSTDOCFLAGS='-D warnings':
    cargo doc --workspace --no-deps --features need-more-recent-msrv
    cargo doc --features=max,lean,small --workspace --no-deps --features need-more-recent-msrv

# Run all unit tests
unit-tests:
    cargo nextest run --no-fail-fast
    cargo nextest run -p gix-testtools --no-fail-fast
    cargo nextest run -p gix-testtools --features xz --no-fail-fast
    cargo nextest run -p gix-archive --no-default-features --no-fail-fast
    cargo nextest run -p gix-archive --features tar --no-fail-fast
    cargo nextest run -p gix-archive --features tar_gz --no-fail-fast
    cargo nextest run -p gix-archive --features zip --no-fail-fast
    cargo nextest run -p gix-status-tests --features gix-features-parallel --no-fail-fast
    cargo nextest run -p gix-worktree-state-tests --features gix-features-parallel --no-fail-fast
    cargo nextest run -p gix-worktree-tests --features gix-features-parallel --no-fail-fast
    cargo nextest run -p gix-object --no-fail-fast
    cargo nextest run -p gix-object --features verbose-object-parsing-errors --no-fail-fast
    cargo nextest run -p gix-tempfile --features signals --no-fail-fast
    cargo nextest run -p gix-features --all-features --no-fail-fast
    cargo nextest run -p gix-ref-tests --all-features --no-fail-fast
    cargo nextest run -p gix-odb --all-features --no-fail-fast
    cargo nextest run -p gix-odb-tests --features gix-features-parallel --no-fail-fast
    cargo nextest run -p gix-pack --all-features --no-fail-fast
    cargo nextest run -p gix-pack-tests --features all-features --no-fail-fast
    cargo nextest run -p gix-pack-tests --features gix-features-parallel --no-fail-fast
    cargo nextest run -p gix-index-tests --features gix-features-parallel --no-fail-fast
    cargo nextest run -p gix-packetline --features blocking-io,maybe-async/is_sync --test blocking-packetline --no-fail-fast
    cargo nextest run -p gix-packetline --features async-io --test async-packetline --no-fail-fast
    cargo nextest run -p gix-transport --features http-client-curl,maybe-async/is_sync --no-fail-fast
    cargo nextest run -p gix-transport --features http-client-reqwest,maybe-async/is_sync --no-fail-fast
    cargo nextest run -p gix-transport --features async-client --no-fail-fast
    cargo nextest run -p gix-protocol --features blocking-client --no-fail-fast
    cargo nextest run -p gix-protocol --features async-client --no-fail-fast
    cargo nextest run -p gix --no-default-features --no-fail-fast
    cargo nextest run -p gix --no-default-features --features basic,comfort,max-performance-safe --no-fail-fast
    cargo nextest run -p gix --no-default-features --features basic,extras,comfort,need-more-recent-msrv --no-fail-fast
    cargo nextest run -p gix --features async-network-client --no-fail-fast
    cargo nextest run -p gix --features blocking-network-client --no-fail-fast
    cargo nextest run -p gitoxide-core --lib --no-tests=warn --no-fail-fast
    cargo test --workspace --doc --no-fail-fast

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
    cargo build -p gix-testtools --bin jtt
    dbg="$({{ j }} dbg)" && tests/journey.sh "$dbg/ein" "$dbg/gix" "$dbg/jtt" max

# Run journey tests (`max-pure`)
journey-tests-pure:
    cargo build --no-default-features --features max-pure
    cargo build -p gix-testtools --bin jtt
    dbg="$({{ j }} dbg)" && tests/journey.sh "$dbg/ein" "$dbg/gix" "$dbg/jtt" max-pure

# Run journey tests (`small`)
journey-tests-small:
    cargo build --no-default-features --features small
    cargo build -p gix-testtools
    dbg="$({{ j }} dbg)" && tests/journey.sh "$dbg/ein" "$dbg/gix" "$dbg/jtt" small

# Run journey tests (`lean-async`)
journey-tests-async:
    cargo build --no-default-features --features lean-async
    cargo build -p gix-testtools
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
        --no-default-features --features async-network-client,max-performance

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
