#!/bin/sh
set -eu

# Consume the request before exiting so capability writes cannot race a closed pipe.
cat >/dev/null

test "$1" = get && \
echo username=user-script && \
echo password=pass-script
