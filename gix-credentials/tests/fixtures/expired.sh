#!/usr/bin/env bash
set -eu

test "$1" = get && \
echo username=user-expired && \
echo password=pass-expired && \
echo password_expiry_utc=1
