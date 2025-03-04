#!/bin/sh
set -e
mkdir empty-context
docker build -f etc/docker/Dockerfile.test-cross-s390x \
    -t cross-rs-gitoxide/s390x-unknown-linux-gnu empty-context
