#!/bin/sh
set -ex

# Build the customized `cross` container image.
docker build -t cross-rs-gitoxide:s390x-unknown-linux-gnu \
    - <etc/docker/Dockerfile.test-cross-s390x

# Clean files that could cause tests to wrongly pass or fail.
cargo clean
gix clean -xd -m '*generated*' -e

# Run the test suite.
cross test --workspace --no-fail-fast --target s390x-unknown-linux-gnu \
    --no-default-features --features max-pure \
    -- --skip realpath::fuzzed_timeout
