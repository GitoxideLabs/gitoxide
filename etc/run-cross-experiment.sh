#!/bin/sh

# Usage:
#
#   etc/run-cross-experiment.sh armv7-linux-androideabi
#   etc/run-cross-experiment.sh s390x-unknown-linux-gnu

set -eux
target="$1"

# Build the customized `cross` container image.
docker build --build-arg "TARGET=$target" -t "cross-rs-gitoxide:$target" \
    - <etc/docker/Dockerfile.test-cross

# Clean files that could cause tests to wrongly pass or fail.
cargo clean
gix clean -xd -m '*generated*' -e

# Run the test suite.
NO_PRELOAD_CXX=1 cross test --workspace --no-fail-fast --target "$target" \
    --no-default-features --features max-pure \
    -- --skip realpath::fuzzed_timeout
