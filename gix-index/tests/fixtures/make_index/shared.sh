#!/usr/bin/env bash

UNTRACKED_CACHE_MTIME="2038-01-19 03:14:07.123456789Z"

seed_untracked_cache_times() {
    touch -d "$UNTRACKED_CACHE_MTIME" "$@"
}
