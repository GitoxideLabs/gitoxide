#!/usr/bin/env -S just --justfile
# ^ A shebang isn't required, but allows a justfile to be executed
#   like a script, with `./justfile test`, for example.

default:
    {{ just_executable() }} --list

alias t := test
alias c := check
alias nt := nextest

# run all tests, clippy, including journey tests, try building docs
test: clippy check doc unit-tests journey-tests-pure journey-tests-small journey-tests-async journey-tests check-mode

# run all tests, without clippy, and try building docs
ci-test: check doc unit-tests check-mode

# run all journey tests - should be run in a fresh clone or after `cargo clean`
ci-journey-tests: journey-tests-pure journey-tests-small journey-tests-async journey-tests

clear-target:
    cargo clean

# Run cargo clippy on all crates
clippy *clippy-args:
    cargo clippy --workspace --all-targets -- {{ clippy-args }}
    cargo clippy --workspace --no-default-features --features small -- {{ clippy-args }}
    cargo clippy --workspace --no-default-features --features max-pure -- {{ clippy-args }}
    cargo clippy --workspace --no-default-features --features lean-async --tests -- {{ clippy-args }}

# Run cargo clippy on all crates, fixing what can be fixed, and format all code
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
    # assure compile error occurs
    if cargo check --features lean-async 2>/dev/null; then false; else true; fi
    if cargo check -p gitoxide-core --all-features 2>/dev/null; then false; else true; fi
    if cargo check -p gix-packetline --all-features 2>/dev/null; then false; else true; fi
    if cargo check -p gix-transport --all-features 2>/dev/null; then false; else true; fi
    if cargo check -p gix-protocol --all-features 2>/dev/null; then false; else true; fi
    cargo tree -p gix --no-default-features -e normal -i imara-diff 2>&1 | grep warning # warning happens if nothing found, no exit code :/
    cargo tree -p gix --no-default-features -e normal -i gix-submodule 2>&1 | grep warning
    cargo tree -p gix --no-default-features -e normal -i gix-pathspec 2>&1 | grep warning
    cargo tree -p gix --no-default-features -e normal -i gix-filter 2>&1 | grep warning
    if cargo tree -p gix --no-default-features -i gix-credentials 2>/dev/null; then false; else true; fi
    cargo check --no-default-features --features lean
    cargo check --no-default-features --features lean-async
    cargo check --no-default-features --features max
    cargo check -p gitoxide-core --features blocking-client
    cargo check -p gitoxide-core --features async-client
    cargo check -p gix-pack --no-default-features
    cargo check -p gix-pack --no-default-features --features generate
    cargo check -p gix-pack --no-default-features --features streaming-input
    cd gix-hash; \
        set -ex; \
        cargo check --all-features; \
        cargo check
    cd gix-object; \
        set -ex; \
        cargo check --all-features; \
        cargo check --features verbose-object-parsing-errors
    cd gix-attributes && cargo check --features serde
    cd gix-glob && cargo check --features serde
    cd gix-worktree; \
        set -ex; \
        cargo check --features serde; \
        cargo check --no-default-features;
    cd gix-actor && cargo check --features serde
    cd gix-date && cargo check --features serde
    cargo check -p gix-tempfile --features signals
    cargo check -p gix-tempfile --features hp-hashmap
    cargo check -p gix-pack --features serde
    cargo check -p gix-pack --features pack-cache-lru-static
    cargo check -p gix-pack --features pack-cache-lru-dynamic
    cargo check -p gix-pack --features object-cache-dynamic
    cargo check -p gix-packetline --features blocking-io
    cargo check -p gix-packetline --features async-io
    cd gix-index && cargo check --features serde
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
    cargo check -p gix-features --features fs-walkdir-parallel
    cargo check -p gix-features --features fs-read-dir
    cargo check -p gix-features --features rustsha1
    cargo check -p gix-features --features fast-sha1
    cargo check -p gix-features --features progress
    cargo check -p gix-features --features io-pipe
    cargo check -p gix-features --features crc32
    cargo check -p gix-features --features zlib
    cargo check -p gix-features --features zlib,zlib-ng
    cargo check -p gix-features --features zlib,zlib-ng-compat
    cargo check -p gix-features --features zlib-stock
    cargo check -p gix-features --features zlib,zlib-stock
    cargo check -p gix-features --features cache-efficiency-debug
    cd gix-commitgraph; \
      set -ex; \
      cargo check --all-features
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
    cargo check -p gix --no-default-features
    cargo check -p gix-odb --features serde
    cargo check --no-default-features --features max-control

# Run cargo doc on all crates
doc $RUSTDOCFLAGS="-D warnings":
    cargo doc --workspace --no-deps --features need-more-recent-msrv
    cargo doc --features=max,lean,small --workspace --no-deps --features need-more-recent-msrv

# run all unit tests
unit-tests:
    cargo nextest run
    cargo test --doc
    cargo nextest run -p gix-testtools
    cargo nextest run -p gix-testtools --features xz
    cargo nextest run -p gix-archive --no-default-features
    cargo nextest run -p gix-archive --features tar
    cargo nextest run -p gix-archive --features tar_gz
    cargo nextest run -p gix-archive --features zip
    cargo nextest run -p gix-status-tests --features "gix-features-parallel"
    cargo nextest run -p gix-worktree-state-tests --features "gix-features-parallel"
    cargo nextest run -p gix-worktree-tests --features "gix-features-parallel"
    cd gix-object; \
        set -ex; \
        cargo nextest run; \
        cargo nextest run --features verbose-object-parsing-errors
    cargo nextest run -p gix-tempfile --features signals
    cargo nextest run -p gix-features --all-features
    cargo nextest run -p gix-ref-tests --all-features
    cargo nextest run -p gix-odb --all-features
    cargo nextest run -p gix-odb-tests --features gix-features-parallel
    cargo nextest run -p gix-pack --all-features
    cargo nextest run -p gix-pack-tests --features all-features
    cargo nextest run -p gix-pack-tests --features "gix-features-parallel"
    cargo nextest run -p gix-index-tests --features "gix-features-parallel"
    cargo nextest run -p gix-packetline --features blocking-io,maybe-async/is_sync --test blocking-packetline
    cargo nextest run -p gix-packetline --features "async-io" --test async-packetline
    cargo nextest run -p gix-transport --features http-client-curl,maybe-async/is_sync
    cargo nextest run -p gix-transport --features http-client-reqwest,maybe-async/is_sync
    cargo nextest run -p gix-transport --features async-client
    cargo nextest run -p gix-protocol --features blocking-client
    cargo nextest run -p gix-protocol --features async-client
    cargo nextest run -p gix --no-default-features
    cargo nextest run -p gix --no-default-features --features basic,extras,comfort,need-more-recent-msrv
    cargo nextest run -p gix --features async-network-client
    cargo nextest run -p gix --features blocking-network-client
    cargo nextest run -p gitoxide-core --lib --no-tests=warn

# These tests aren't run by default as they are flaky (even locally)
unit-tests-flaky:
    cargo test -p gix --features async-network-client-async-std

target_dir := `cargo metadata --format-version 1 | jq -r .target_directory`
ein := target_dir / "debug/ein"
gix := target_dir / "debug/gix"
jtt := target_dir / "debug/jtt"
it := target_dir / "debug/it"

# run journey tests (max)
journey-tests:
    cargo build --features http-client-curl-rustls
    cargo build -p gix-testtools --bin jtt
    ./tests/journey.sh {{ ein }} {{ gix }} {{ jtt }} max

# run journey tests (max-pure)
journey-tests-pure:
    cargo build --no-default-features --features max-pure
    cargo build -p gix-testtools --bin jtt
    ./tests/journey.sh {{ ein }} {{ gix }} {{ jtt }} max-pure

# run journey tests (small)
journey-tests-small:
    cargo build --no-default-features --features small
    cargo build -p gix-testtools
    ./tests/journey.sh {{ ein }} {{ gix }} {{ jtt }} small

# run journey tests (lean-async)
journey-tests-async:
    cargo build --no-default-features --features lean-async
    cargo build -p gix-testtools
    ./tests/journey.sh {{ ein }} {{ gix }} {{ jtt }} async

# Build a customized `cross` container image for testing
cross-image target:
    docker build --build-arg "TARGET={{ target }}" -t "cross-rs-gitoxide:{{ target }}" \
        - <etc/docker/Dockerfile.test-cross

# Test another platform with `cross`
cross-test target: (cross-image target)
    NO_PRELOAD_CXX=1 cross test --workspace --no-fail-fast --target "{{ target }}" \
        --no-default-features --features max-pure \
        -- --skip realpath::fuzzed_timeout

# Test s390x with `cross`
cross-test-s390x: (cross-test 's390x-unknown-linux-gnu')

# Test Android with `cross` (limited)
cross-test-android: (cross-test 'armv7-linux-androideabi')

# Run cargo-diet on all crates to see that they are still in bound
check-size:
    ./etc/check-package-size.sh

# Check the minimal support rust version for currently installed Rust version
ci-check-msrv:
    rustc --version
    cargo check -p gix
    cargo check -p gix --no-default-features --features async-network-client,max-performance

# Enter a nix-shell able to build on macos
nix-shell-macos:
    nix-shell -p pkg-config openssl libiconv darwin.apple_sdk.frameworks.Security darwin.apple_sdk.frameworks.SystemConfiguration

# run various auditing tools to assure we are legal and safe
audit:
    cargo deny check advisories bans licenses sources

# run tests with `cargo nextest` (all unit-tests, no doc-tests, faster)
nextest *FLAGS="--workspace":
    cargo nextest run {{ FLAGS }}

summarize EXPRESSION="all()": (nextest "--workspace --run-ignored all --no-fail-fast --status-level none --final-status-level none -E '" EXPRESSION "'")

# run nightly rustfmt for its extra features, but check that it won't upset stable rustfmt
fmt:
    cargo +nightly fmt --all -- --config-path rustfmt-nightly.toml
    cargo +stable fmt --all -- --check
    just --fmt --unstable

# Cancel this after the first few seconds, as yanked crates will appear in warnings.
find-yanked:
    cargo install --debug --locked --no-default-features --features max-pure --path .

# Find shell scripts whose +x/-x bits and magic bytes (e.g. `#!`) disagree
check-mode:
    cargo build -p internal-tools
    "{{ it }}" check-mode

# Delete gix-packetline-blocking/src and regenerate from gix-packetline/src
copy-packetline:
    ./etc/copy-packetline.sh
